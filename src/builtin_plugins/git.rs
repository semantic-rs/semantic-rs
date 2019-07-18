use std::cell::RefCell;
use std::env;
use std::ops::Try;

use failure::Fail;
use git2::{self, Oid, Repository, Signature, RemoteCallbacks, PushOptions, Cred};
use serde::{Deserialize, Serialize};

use crate::config::CfgMapExt;
use crate::plugin::proto::{
    request,
    response::{self, PluginResponse, PluginResponseBuilder},
};
use crate::plugin::proto::{GitRevision, Version};
use crate::plugin::{PluginInterface, PluginStep};
use std::path::Path;

pub struct GitPlugin {
    state: RefCell<GitPluginState>,
}

enum GitPluginState {
    Uninit,
    Init(GitPluginStateData),
}

impl GitPluginState {
    pub fn is_initialized(&self) -> bool {
        match self {
            GitPluginState::Init(_) => true,
            GitPluginState::Uninit => false,
        }
    }

    pub fn as_initialized(&self) -> &GitPluginStateData {
        match self {
            GitPluginState::Init(data) => data,
            GitPluginState::Uninit => {
                panic!("GitPluginState must be initialized before calling `as_initialized`")
            }
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct GitPluginConfig {
    user_name: Option<String>,
    user_email: Option<String>,
    #[serde(default = "default_branch")]
    branch: String,
    #[serde(default = "default_remote")]
    remote: String,
    #[serde(default)]
    force_https: bool,
}

fn default_branch() -> String {
    "master".into()
}

fn default_remote() -> String {
    "origin".into()
}

struct GitPluginStateData {
    config: GitPluginConfig,
    repo: Repository,
    signature: Signature<'static>,
}

impl GitPluginStateData {
    pub fn new(config: GitPluginConfig, repo: Repository) -> Result<Self, failure::Error> {
        let signature = Self::get_signature(&config, &repo)?;
        Ok(GitPluginStateData {
            config,
            repo,
            signature,
        })
    }

    pub fn get_signature(
        cfg: &GitPluginConfig,
        repo: &Repository,
    ) -> Result<Signature<'static>, failure::Error> {
        let author = {
            if let Some(author) = cfg.user_name.clone() {
                author
            } else {
                let mut author = env::var("GIT_COMMITTER_NAME").map_err(failure::Error::from);

                if author.is_err() {
                    let config = repo.config()?;
                    author = config
                        .get_string("user.name")
                        .map_err(|_| GitPluginError::CommitterNameUndefined)
                        .map_err(failure::Error::from);
                }

                author?
            }
        };

        let email = {
            if let Some(email) = cfg.user_email.clone() {
                email
            } else {
                let mut email = env::var("GIT_COMMITTER_EMAIL").map_err(failure::Error::from);

                if email.is_err() {
                    let config = repo.config()?;
                    email = config
                        .get_string("user.email")
                        .map_err(|_| GitPluginError::CommitterEmailUndefined)
                        .map_err(failure::Error::from);
                }

                email?
            }
        };

        Ok(Signature::now(&author, &email)?)
    }

    pub fn perform_pre_flight_checks<T>(&self, response: &mut PluginResponseBuilder<T>) {
        let result = || -> Result<(), failure::Error> {
            let remote = self.repo.find_remote(&self.config.remote)?;
            let remote_url = remote.url().ok_or(GitPluginError::GitRemoteUndefined)?;

            if !self.config.force_https && is_https_remote(remote_url) {
                response.warnings(&[
                    "Git remote is not HTTPS and 'cfg.git.force_https' != true:",
                    "The publishing will fail if your environment doesn't hold your git ssh keys",
                    "Consider setting 'cfg.git.force_https = true', that's most likely what you want if you're using GH_TOKEN authentication",
                ]);
            }

            Ok(())
        }();

        if let Err(err) = result {
            response.error(err);
        }
    }

    fn perform_pre_flight_overrides(&mut self) -> Result<(), failure::Error> {
        if self.config.force_https {
            let remote_name = self.config.remote.clone();
            let remote_url = self.repo
                .find_remote(&remote_name)?
                .url()
                .map(str::to_string)
                .ok_or(GitPluginError::GitRemoteUndefined)?;

            if !is_https_remote(&remote_url) {
                // TODO: replace with generic regex
                let rules = [
                    ("git@github.com:", "https://github.com/"),
                    ("git://github.com/", "https://github.com/"),
                ];

                let mut new_url = None;

                for (pattern, substitute) in &rules {
                    if remote_url.starts_with(pattern) {
                        new_url = Some(remote_url.replace(pattern, substitute));
                        break;
                    }
                }

                let url = new_url.ok_or(GitPluginError::RemoteNotSupportedForHttpsForcing(remote_url))?;

                self.set_remote_url(&url)?;
            }
        }

        Ok(())
    }

    fn set_remote_url(&mut self, url: &str) -> Result<(), failure::Error> {
        self.repo.remote_set_url(&self.config.remote, url)?;
        self.repo.remote_set_pushurl(&self.config.remote, Some(url))?;
        Ok(())
    }

    fn commit_files(&self, files: &[String], commit_msg: &str) -> Result<(), failure::Error> {
        let files = files
            .iter()
            .filter(|filename| {
                let path = Path::new(filename);
                !self.repo
                    .status_should_ignore(path)
                    .expect("Determining ignore status of file failed")
            });

        self.add(files)?;

        self.commit(&commit_msg)?;

        Ok(())
    }

    fn add<P: AsRef<Path>>(&self, files: impl Iterator<Item = P>) -> Result<(), git2::Error> {
        let mut index = self.repo.index()?;

        for path in files {
            index.add_path(path.as_ref())?;
        }

        index.write()
    }

    fn commit(&self, message: &str) -> Result<(), git2::Error> {
        let update_ref = format!("refs/heads/{}", self.config.branch);

        let oid = self.repo.refname_to_id("HEAD")?;
        let parent_commit = self.repo.find_commit(oid)?;
        let parents = vec![&parent_commit];

        let mut index = self.repo.index()?;
        let tree_oid = index.write_tree()?;
        let tree = self.repo.find_tree(tree_oid)?;

        self.repo.commit(
            Some(&update_ref),
            &self.signature,
            &self.signature,
            message,
            &tree,
            &parents,
        ).map(|_| ())
    }

    fn create_tag(&self, tag_name: &str, message: &str) -> Result<(), git2::Error> {
        let rev = format!("refs/heads/{}", self.config.branch);
        let obj = self.repo.revparse_single(&rev)?;

        self.repo.tag(tag_name, &obj, &self.signature, message, false)
            .map(|_| ())
    }


    pub fn push(&self, tag_name: &str) -> Result<(), failure::Error> {
        let repo = &self.repo;

        let branch = &self.config.branch;
        let token = std::env::var("GH_TOKEN").ok();

        // We need to push both the branch we just committed as well as the tag we created.
        let branch_ref = format!("refs/heads/{}", branch);
        let tag_ref = format!("refs/tags/{}", tag_name);
        let refs = [&branch_ref[..], &tag_ref[..]];

        let mut remote = repo.find_remote(&self.config.remote)?;
        let remote_url = remote.url().ok_or(GitPluginError::GitRemoteUndefined)?;
        let mut cbs = RemoteCallbacks::new();
        let mut opts = PushOptions::new();

        if is_https_remote(remote_url) {
            let token = token.ok_or(GitPluginError::GithubTokenUndefined)?;
            cbs.credentials(move |_url, _username, _allowed| Cred::userpass_plaintext(&token, ""));
            opts.remote_callbacks(cbs);
        } else {
            cbs.credentials(|_url, username, _allowed| Cred::ssh_key_from_agent(&username.unwrap()));
            opts.remote_callbacks(cbs);
        }

        remote.push(&refs, Some(&mut opts))?;

        Ok(())
    }

    fn latest_tag(&self) -> Option<(GitRevision, semver::Version)> {
        let tags = self.repo.tag_names(None).ok()?;

        let opt_version = tags
            .iter()
            .filter_map(|tag| tag.and_then(|s| semver::Version::parse(&s[1..]).ok()))
            .max();

        if let Some(version) = opt_version {
            let tag_name = format!("v{}", version);
            Some((tag_name, version))
        } else {
            None
        }
    }

    fn earliest_revision(&self) -> Result<Oid, failure::Error> {
        let mut revwalk = self.repo.revwalk()?;
        revwalk.push_head()?;

        let earliest_commit = revwalk
            .last()
            .expect("failed to find the earliest revision")?;

        Ok(earliest_commit)
    }

}

impl GitPlugin {
    pub fn new() -> Self {
        GitPlugin {
            state: RefCell::new(GitPluginState::Uninit),
        }
    }
}

impl PluginInterface for GitPlugin {
    fn methods(&self, _req: request::Methods) -> response::Methods {
        let methods = vec![
            PluginStep::PreFlight,
            PluginStep::GetLastRelease,
            PluginStep::Commit,
        ];
        PluginResponse::from_ok(methods)
    }

    fn pre_flight(&self, params: request::PreFlight) -> response::PreFlight {
        let mut response = PluginResponse::builder();

        let config = {
            let config_toml = params.cfg_map.get_sub_table("git")?;
            toml::Value::Table(config_toml).try_into()?
        };

        log::debug!("git(config): {:?}", config);

        let mut data = {
            let path = params.cfg_map.project_root()?;
            let repo = Repository::open(path)?;
            GitPluginStateData::new(config, repo)?
        };

        data.perform_pre_flight_checks(&mut response);
        data.perform_pre_flight_overrides()?;

        log::debug!("git(pre_flight): finished");

        self.state.replace(GitPluginState::Init(data));

        response.body(()).build()
    }

    fn get_last_release(&self, _params: request::GetLastRelease) -> response::GetLastRelease {
        let state_bind = self.state.borrow();
        let state = state_bind.as_initialized();

        let version = match state.latest_tag() {
            Some((rev, version)) => Version {
                rev,
                semver: Some(version),
            },
            None => {
                let earliest_commit = state.earliest_revision()?;
                Version {
                    rev: earliest_commit.to_string(),
                    semver: None,
                }
            }
        };

        PluginResponse::from_ok(version)
    }

    fn commit(&self, params: request::Commit) -> response::Commit {
        let data = params.data;

        let state_bind = self.state.borrow();
        let state = state_bind.as_initialized();

        // TODO: make releaserc-configurable
        let commit_msg = format!("chore(release): Version {} [skip ci]", data.version);
        let tag_name = format!("v{}", data.version);

        log::info!("git: committing files {:?}", data.files_to_commit);
        state.commit_files(&data.files_to_commit, &commit_msg)?;
        log::info!("git: creating tag {:?}", tag_name);
        state.create_tag(&tag_name, &data.changelog)?;
        log::info!("git: pushing changes");
        state.push(&tag_name)?;

        PluginResponse::from_ok(tag_name)
    }
}

#[derive(Fail, Debug)]
pub enum GitPluginError {
    #[fail(
        display = "committer name was not found in [env::GIT_COMMITTER_NAME, releaserc.cfg.git.user_name, git config user.name]"
    )]
    CommitterNameUndefined,
    #[fail(
        display = "committer email was not found in [env::GIT_COMMITTER_EMAIL, releaserc.cfg.git.user_email, git config user.email]"
    )]
    CommitterEmailUndefined,
    #[fail(display = "failed to determine git remote url")]
    GitRemoteUndefined,
    #[fail(display = "GH_TOKEN is undefined: cannot push changes")]
    GithubTokenUndefined,
    #[fail(display = "{} is not supported for https forcing, please consider opening an issue at https://github.com/etclabscore/semantic-rs/issues/new/choose", _0)]
    RemoteNotSupportedForHttpsForcing(String),
}

fn is_https_remote(remote: &str) -> bool {
    remote.starts_with("https://")
}

