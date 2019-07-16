use crate::config::{CfgMap, CfgMapExt};
use crate::plugin::proto::request::{
    CommitRequest, GetLastReleaseRequest, MethodsRequest, PluginRequest, PreFlightRequest,
};
use crate::plugin::proto::response::{
    CommitResponse, GetLastReleaseResponse, MethodsResponse, PluginResponse, PluginResponseBuilder,
    PluginResult, PreFlightResponse,
};
use crate::plugin::{PluginInterface, PluginStep};

use std::collections::HashMap;

use crate::plugin::proto::{GitRevision, Version};
use crate::plugin::PluginStep::VerifyRelease;
use failure::Fail;
use git2::{self, Commit, Cred, Oid, PushOptions, Remote, RemoteCallbacks, Repository, Signature};
use hubcaps::repositories::Repo;
use serde::{Deserialize, Serialize};
use std::cell::RefCell;
use std::env;
use std::path::Path;
use toml::map::Map;

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
    branch: Option<String>,
    #[serde(default = "default_remote")]
    remote: String,
    #[serde(default)]
    force_https: bool,
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
}

impl GitPlugin {
    pub fn new() -> Self {
        GitPlugin {
            state: RefCell::new(GitPluginState::Uninit),
        }
    }
}

impl PluginInterface for GitPlugin {
    fn methods(&self, req: MethodsRequest) -> PluginResult<MethodsResponse> {
        let mut methods = HashMap::new();
        methods.insert(PluginStep::PreFlight, true);
        methods.insert(PluginStep::GetLastRelease, true);
        methods.insert(PluginStep::Commit, true);
        let resp = PluginResponse::builder().body(methods).build();
        Ok(resp)
    }

    fn pre_flight(&self, params: PreFlightRequest) -> PluginResult<PreFlightResponse> {
        let mut response = PluginResponse::builder();

        let config_result = || -> Result<GitPluginConfig, failure::Error> {
            let config_toml = params.cfg_map.get_sub_table("git")?;
            Ok(toml::Value::Table(config_toml).try_into()?)
        }();

        let config = match config_result {
            Ok(data) => data,
            Err(err) => {
                response.error(err);
                return Ok(response.build());
            }
        };

        log::debug!("git(config): {:?}", config);

        let data_result = || -> Result<GitPluginStateData, failure::Error> {
            let path = params.cfg_map.project_root()?;
            let repo = Repository::open(path)?;
            Ok(GitPluginStateData::new(config, repo)?)
        }();

        let data = match data_result {
            Ok(data) => data,
            Err(err) => {
                response.error(err);
                return Ok(response.build());
            }
        };

        data.perform_pre_flight_checks(&mut response);

        log::debug!("git(pre_flight): finished");

        self.state.replace(GitPluginState::Init(data));

        Ok(response.body(()).build())
    }

    fn get_last_release(
        &self,
        params: GetLastReleaseRequest,
    ) -> PluginResult<GetLastReleaseResponse> {
        let mut response = PluginResponse::builder();

        let data_bind = self.state.borrow();
        let data = data_bind.as_initialized();

        let version = match latest_tag(&data.repo) {
            Some((rev, version)) => Version::Semver(rev.to_string(), version),
            None => {
                let earliest_commit = match earliest_revision(&data.repo) {
                    Ok(oid) => oid,
                    Err(err) => {
                        response.error(err);
                        return Ok(response.build());
                    }
                };
                Version::None(earliest_commit.to_string())
            }
        };

        Ok(response.body(version).build())
    }

    fn commit(&self, params: CommitRequest) -> PluginResult<CommitResponse> {
        unimplemented!()
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
}

fn is_https_remote(remote: &str) -> bool {
    remote.starts_with("https://")
}

fn latest_tag(repo: &Repository) -> Option<(GitRevision, semver::Version)> {
    let tags = repo.tag_names(None).ok()?;

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

fn earliest_revision(repo: &Repository) -> Result<Oid, failure::Error> {
    let mut revwalk = repo.revwalk()?;
    revwalk.push_head();

    let earliest_commit = revwalk
        .last()
        .expect("failed to find the earliest revision")?;

    Ok(earliest_commit)
}
