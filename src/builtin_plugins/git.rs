use std::env;
use std::ops::Try;

use failure::Fail;
use git2::{self, Cred, Oid, PushOptions, RemoteCallbacks, Repository, Signature};
use serde::{Deserialize, Serialize};

use crate::plugin_support::flow::{Availability, FlowError, ProvisionCapability, Value};
use crate::plugin_support::keys::{
    CURRENT_VERSION, FILES_TO_COMMIT, GIT_BRANCH, GIT_REMOTE, GIT_REMOTE_URL, NEXT_VERSION, PROJECT_ROOT, RELEASE_NOTES,
};
use crate::plugin_support::proto::response::{self, PluginResponse, PluginResponseBuilder};
use crate::plugin_support::proto::{GitRevision, Version};
use crate::plugin_support::{PluginInterface, PluginStep};
use std::path::Path;

pub struct GitPlugin {
    config: Config,
    state: Option<State>,
}

struct State {
    repo: Repository,
    signature: Signature<'static>,
    current_version: Option<Version>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
struct Config {
    user_name: Value<Option<String>>,
    user_email: Value<Option<String>>,
    branch: Value<String>,
    remote: Value<String>,
    force_https: Value<bool>,
    project_root: Value<String>,
    next_version: Value<semver::Version>,
    files_to_commit: Value<Vec<String>>,
    changelog: Value<String>,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            user_name: Value::builder("user_name").default_value().build(),
            user_email: Value::builder("user_email").default_value().build(),
            branch: Value::builder("branch").value(default_branch()).build(),
            remote: Value::builder("remote").value(default_remote()).build(),
            force_https: Value::builder("force_https").default_value().build(),
            project_root: Value::builder(PROJECT_ROOT).protected().build(),
            next_version: Value::builder(NEXT_VERSION)
                .protected()
                .required_at(PluginStep::Commit)
                .build(),
            files_to_commit: Value::builder(FILES_TO_COMMIT)
                .protected()
                .required_at(PluginStep::Commit)
                .build(),
            changelog: Value::builder(RELEASE_NOTES)
                .protected()
                .required_at(PluginStep::Commit)
                .build(),
        }
    }
}

fn default_branch() -> String {
    "master".into()
}

fn default_remote() -> String {
    "origin".into()
}

impl State {
    pub fn new(config: &Config, repo: Repository) -> Result<Self, failure::Error> {
        let signature = Self::get_signature(&config, &repo)?;
        Ok(State {
            repo,
            signature,
            current_version: None,
        })
    }

    pub fn get_signature(cfg: &Config, repo: &Repository) -> Result<Signature<'static>, failure::Error> {
        let author = {
            if let Some(author) = cfg.user_name.as_value().clone() {
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
            if let Some(email) = cfg.user_email.as_value().clone() {
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

    pub fn perform_pre_flight_checks<T>(&self, config: &Config, response: &mut PluginResponseBuilder<T>) {
        let result = || -> Result<(), failure::Error> {
            let remote = self.repo.find_remote(&config.remote.as_value())?;
            let remote_url = remote.url().ok_or(GitPluginError::GitRemoteUndefined)?;

            if !config.force_https.as_value() && is_https_remote(remote_url) {
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

    fn perform_pre_flight_overrides(&mut self, config: &Config) -> Result<(), failure::Error> {
        if *config.force_https.as_value() {
            let remote_name = config.remote.as_value().clone();
            let remote_url = self
                .repo
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

                self.set_remote_url(config, &url)?;
            }
        }

        Ok(())
    }

    fn set_remote_url(&mut self, config: &Config, url: &str) -> Result<(), failure::Error> {
        self.repo.remote_set_url(&config.remote.as_value(), url)?;
        self.repo.remote_set_pushurl(&config.remote.as_value(), Some(url))?;
        Ok(())
    }

    fn commit_files(&self, config: &Config, files: &[String], commit_msg: &str) -> Result<(), failure::Error> {
        let files = files.iter().filter(|filename| {
            let path = Path::new(filename);
            !self
                .repo
                .status_should_ignore(path)
                .expect("Determining ignore status of file failed")
        });

        self.add(files)?;

        self.commit(config, &commit_msg)?;

        Ok(())
    }

    fn add<P: AsRef<Path>>(&self, files: impl Iterator<Item = P>) -> Result<(), git2::Error> {
        let mut index = self.repo.index()?;

        for path in files {
            index.add_path(path.as_ref())?;
        }

        index.write()
    }

    fn commit(&self, config: &Config, message: &str) -> Result<(), git2::Error> {
        let update_ref = format!("refs/heads/{}", config.branch.as_value());

        let oid = self.repo.refname_to_id("HEAD")?;
        let parent_commit = self.repo.find_commit(oid)?;
        let parents = vec![&parent_commit];

        let mut index = self.repo.index()?;
        let tree_oid = index.write_tree()?;
        let tree = self.repo.find_tree(tree_oid)?;

        self.repo
            .commit(
                Some(&update_ref),
                &self.signature,
                &self.signature,
                message,
                &tree,
                &parents,
            )
            .map(|_| ())
    }

    fn create_tag(&self, config: &Config, tag_name: &str, message: &str) -> Result<(), git2::Error> {
        let rev = format!("refs/heads/{}", config.branch.as_value());
        let obj = self.repo.revparse_single(&rev)?;

        self.repo
            .tag(tag_name, &obj, &self.signature, message, false)
            .map(|_| ())
    }

    pub fn push(&self, config: &Config, tag_name: &str) -> Result<(), failure::Error> {
        let repo = &self.repo;

        let branch = config.branch.as_value();
        let remote = config.remote.as_value();
        let token = std::env::var("GH_TOKEN").ok();

        // We need to push both the branch we just committed as well as the tag we created.
        let branch_ref = format!("refs/heads/{}", branch);
        let tag_ref = format!("refs/tags/{}", tag_name);
        let refs = [&branch_ref[..], &tag_ref[..]];

        let mut remote = repo.find_remote(remote)?;
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

        let earliest_commit = revwalk.last().expect("failed to find the earliest revision")?;

        Ok(earliest_commit)
    }
}

impl GitPlugin {
    pub fn new() -> Self {
        GitPlugin {
            config: Config::default(),
            state: None,
        }
    }
}

impl PluginInterface for GitPlugin {
    fn name(&self) -> response::Name {
        PluginResponse::from_ok("git".into())
    }

    fn provision_capabilities(&self) -> response::ProvisionCapabilities {
        PluginResponse::from_ok(vec![
            ProvisionCapability::builder(GIT_BRANCH)
                .after_step(PluginStep::PreFlight)
                .build(),
            ProvisionCapability::builder(GIT_REMOTE)
                .after_step(PluginStep::PreFlight)
                .build(),
            ProvisionCapability::builder(GIT_REMOTE_URL)
                .after_step(PluginStep::PreFlight)
                .build(),
            ProvisionCapability::builder(CURRENT_VERSION)
                .after_step(PluginStep::GetLastRelease)
                .build(),
            ProvisionCapability::builder("release_tag")
                .after_step(PluginStep::Commit)
                .build(),
        ])
    }

    fn get_value(&self, key: &str) -> response::GetValue {
        let value = match key {
            "git_branch" => serde_json::to_value(self.config.branch.as_value())?,
            "git_remote" => serde_json::to_value(self.config.remote.as_value())?,
            "git_remote_url" => {
                let state = self.state.as_ref().ok_or(GitPluginError::StateIsNone)?;
                let remote = state.repo.find_remote(self.config.remote.as_value())?;
                if let Some(url) = remote.url() {
                    serde_json::to_value(url)?
                } else {
                    return PluginResponse::from_error(GitPluginError::GitRemoteUndefined.into());
                }
            }
            "current_version" => serde_json::to_value(
                self.state
                    .as_ref()
                    .and_then(|s| s.current_version.as_ref())
                    .ok_or_else(|| {
                        FlowError::DataNotAvailableYet(
                            key.to_owned(),
                            Availability::AfterStep(PluginStep::GetLastRelease),
                        )
                    })?,
            )?,
            "release_tag" => serde_json::to_value(format!("v{}", self.config.next_version.as_value()))?,
            other => return PluginResponse::from_error(FlowError::KeyNotSupported(other.to_owned()).into()),
        };

        PluginResponse::from_ok(value)
    }

    fn get_config(&self) -> response::Config {
        PluginResponse::from_ok(serde_json::to_value(&self.config)?)
    }

    fn set_config(&mut self, config: serde_json::Value) -> response::Null {
        self.config = serde_json::from_value(config)?;
        PluginResponse::from_ok(())
    }

    fn methods(&self) -> response::Methods {
        let methods = vec![PluginStep::PreFlight, PluginStep::GetLastRelease, PluginStep::Commit];
        PluginResponse::from_ok(methods)
    }

    fn pre_flight(&mut self) -> response::Null {
        let mut response = PluginResponse::builder();

        let config = &self.config;

        log::debug!("git(config): {:?}", config);

        let mut data = {
            let path = config.project_root.as_value();
            let repo = Repository::open(path)?;
            State::new(config, repo)?
        };

        data.perform_pre_flight_checks(config, &mut response);
        data.perform_pre_flight_overrides(config)?;

        log::debug!("git(pre_flight): finished");

        self.state = Some(data);

        response.body(())
    }

    fn get_last_release(&mut self) -> response::Null {
        let state = self.state.as_mut().ok_or(GitPluginError::StateIsNone)?;

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

        state.current_version.replace(version);

        PluginResponse::from_ok(())
    }

    fn commit(&mut self) -> response::Null {
        let next_version = self.config.next_version.as_value();
        let files_to_commit = self.config.files_to_commit.as_value();
        let changelog = self.config.changelog.as_value();
        let state = self.state.as_ref().ok_or(GitPluginError::StateIsNone)?;
        let config = &self.config;

        // TODO: make releaserc-configurable
        let commit_msg = format!("chore(release): Version {} [skip ci]", next_version);
        let tag_name = format!("v{}", next_version);

        log::info!("Committing files {:?}", files_to_commit);
        state.commit_files(config, &files_to_commit, &commit_msg)?;
        log::info!("Creating tag {:?}", tag_name);
        state.create_tag(config, &tag_name, &changelog)?;
        log::info!("Pushing changes, please wait...");
        state.push(config, &tag_name)?;

        PluginResponse::from_ok(())
    }
}

#[derive(Fail, Debug)]
pub enum GitPluginError {
    #[fail(display = "state is not initialized (forgot to run pre_flight step?)")]
    StateIsNone,
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
    #[fail(
        display = "{} is not supported for https forcing, please consider opening an issue at https://github.com/etclabscore/semantic-rs/issues/new/choose",
        _0
    )]
    RemoteNotSupportedForHttpsForcing(String),
}

fn is_https_remote(remote: &str) -> bool {
    remote.starts_with("https://")
}
