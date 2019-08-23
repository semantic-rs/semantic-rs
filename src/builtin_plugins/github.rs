use std::fmt::Write as _;
use std::ops::Try;
use std::path::{Path, PathBuf};

use failure::Error;
use http::header::HeaderValue;
use hubcaps::releases::ReleaseOptions;
use hubcaps::{Credentials, Github};
use serde::{Deserialize, Serialize};
use tokio::runtime::current_thread::block_on_all;
use url::{ParseError, Url};

use crate::plugin_support::flow::{FlowError, Value};
use crate::plugin_support::keys::{GIT_BRANCH, GIT_REMOTE, GIT_REMOTE_URL, PROJECT_ROOT};
use crate::plugin_support::proto::response::{self, PluginResponse};
use crate::plugin_support::{PluginInterface, PluginStep};
use crate::utils::ResultExt;

const USERAGENT: &str = concat!("semantic-rs/", env!("CARGO_PKG_VERSION"));

pub struct GithubPlugin {
    config: Config,
}

impl GithubPlugin {
    pub fn new() -> Self {
        GithubPlugin {
            config: Config::default(),
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Config {
    assets: Value<Vec<String>>,
    user: Value<Option<String>>,
    repository: Value<Option<String>>,
    remote: Value<String>,
    remote_url: Value<String>,
    branch: Value<String>,
    tag_name: Value<String>,
    changelog: Value<String>,
    draft: Value<bool>,
    pre_release: Value<bool>,
    project_root: Value<String>,
    token: Value<String>,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            assets: Value::builder("assets").default_value().build(),
            user: Value::builder("user").default_value().build(),
            repository: Value::builder("repository").default_value().build(),
            remote: Value::builder(GIT_REMOTE).build(),
            remote_url: Value::builder(GIT_REMOTE_URL).build(),
            branch: Value::builder(GIT_BRANCH).build(),
            tag_name: Value::builder("release_tag").required_at(PluginStep::Publish).build(),
            changelog: Value::builder("release_notes").required_at(PluginStep::Publish).build(),
            draft: Value::builder("draft").default_value().build(),
            pre_release: Value::builder("draft").value(true).build(),
            project_root: Value::builder(PROJECT_ROOT).protected().build(),
            token: Value::builder("GH_TOKEN").load_from_env().build(),
        }
    }
}

fn globs_to_assets<'a>(globs: impl Iterator<Item = &'a str>) -> Vec<Result<Asset, failure::Error>> {
    let mut results = Vec::new();

    for pattern in globs {
        let paths = match glob::glob(pattern) {
            Ok(paths) => paths,
            Err(err) => {
                results.push(Err(err.into()));
                continue;
            }
        };

        for path in paths {
            let path = match path {
                Ok(path) => path,
                Err(err) => {
                    results.push(Err(err.into()));
                    continue;
                }
            };

            let asset_result = Asset::from_path(path);
            results.push(asset_result);
        }
    }

    results
}

impl PluginInterface for GithubPlugin {
    fn name(&self) -> response::Name {
        PluginResponse::from_ok("github".into())
    }

    fn provision_capabilities(&self) -> response::ProvisionCapabilities {
        PluginResponse::from_ok(vec![])
    }

    fn get_value(&self, key: &str) -> response::GetValue {
        PluginResponse::from_error(FlowError::KeyNotSupported(key.to_owned()).into())
    }

    fn get_config(&self) -> response::Config {
        PluginResponse::from_ok(serde_json::to_value(&self.config)?)
    }

    fn set_config(&mut self, config: serde_json::Value) -> response::Null {
        self.config = serde_json::from_value(config)?;
        PluginResponse::from_ok(())
    }

    fn methods(&self) -> response::Methods {
        let methods = vec![PluginStep::PreFlight, PluginStep::Publish];
        PluginResponse::from_ok(methods)
    }

    fn pre_flight(&mut self) -> response::Null {
        let mut response = PluginResponse::builder();
        // Try to parse config
        let config = &self.config;

        // Try to parse assets
        let errors = globs_to_assets(config.assets.as_value().iter().map(String::as_str))
            .into_iter()
            .inspect(|asset| {
                if let Ok(asset) = asset {
                    log::info!("Would upload {} ({})", asset.path().display(), asset.content_type());
                }
            })
            .flat_map(Result::err)
            .collect::<Vec<_>>();

        if errors.is_empty() {
            response.body(())
        } else {
            let mut buffer = String::new();
            writeln!(&mut buffer, "Couldn't process the asset list:")?;
            for error in errors {
                writeln!(&mut buffer, "\t{}", error)?;
            }
            let error_msg = failure::err_msg(buffer);
            response.error(error_msg)
        }
    }

    fn publish(&mut self) -> response::Null {
        let cfg = &self.config;

        let remote_url = self.config.remote_url.as_value();

        let (derived_name, derived_repo) = user_repo_from_url(remote_url)?;

        let user = cfg.user.as_value().as_ref().unwrap_or(&derived_name);
        let repo_name = cfg.repository.as_value().as_ref().unwrap_or(&derived_repo);
        let branch = cfg.branch.as_value();
        let tag_name = cfg.tag_name.as_value();
        let changelog = cfg.changelog.as_value();
        let token = cfg.token.as_value();

        // Create release
        let credentials = Credentials::Token(token.to_owned());

        let release_opts = ReleaseOptions::builder(tag_name)
            .name(tag_name)
            .body(changelog)
            .commitish(branch)
            .draft(*cfg.draft.as_value())
            .prerelease(*cfg.pre_release.as_value())
            .build();

        let release = block_on_all(futures::lazy(move || {
            let github = Github::new(USERAGENT, credentials);
            let repo = github.repo(user, repo_name);
            let releases = repo.releases();
            releases.create(&release_opts)
        }))
        .sync()?;

        // Upload assets
        let token_header_value = HeaderValue::from_str(&format!("token {}", token)).unwrap();

        let mut errored = false;

        let assets = globs_to_assets(cfg.assets.as_value().iter().map(String::as_str))
            .into_iter()
            .collect::<Result<Vec<_>, _>>()?;

        let endpoint_template = format!(
            "https://uploads.github.com/repos/{}/{}/releases/{}/assets?name=",
            user, repo_name, release.id,
        );

        for asset in assets {
            let endpoint = endpoint_template.clone() + asset.name();

            log::info!("Uploading {}, mime-type {}", asset.name(), asset.content_type());
            log::debug!("Upload url: {}", endpoint);

            let body = std::fs::read(asset.path())?;

            let endpoint_url = reqwest::Url::parse(&endpoint)?;
            let content_type_header_value = HeaderValue::from_str(asset.content_type())?;

            let mut response = reqwest::Client::new()
                .post(endpoint_url)
                .body(body)
                .header("Authorization", token_header_value.clone())
                .header("Content-Type", content_type_header_value)
                .send()?;

            if !response.status().is_success() {
                let json: serde_json::Value = response.json()?;
                log::error!("failed to upload asset {}", asset.name());
                log::error!("GitHub response: {:#?}", json);
                errored = true;
            }
        }

        if errored {
            return PluginResponse::from_error(failure::err_msg("failed to upload some assets"));
        }

        PluginResponse::from_ok(())
    }
}

#[derive(Clone, Debug)]
pub struct Asset {
    path: PathBuf,
    name: String,
    content_type: String,
}

impl Asset {
    pub fn from_path(path: impl AsRef<Path>) -> Result<Self, Error> {
        let path = path.as_ref().to_path_buf();

        // Check if path exists
        if !path.exists() {
            return Err(failure::format_err!("asset file not found at {}", path.display()));
        }

        // Check is asset is file
        if !path.is_file() {
            return Err(failure::format_err!("asset at {} is not a file", path.display()));
        }

        // Create a name from the file path
        let name = path
            .file_name()
            .ok_or_else(|| failure::format_err!("couldn't get a file stem for {}", path.display()))?
            .to_str()
            .ok_or_else(|| failure::format_err!("{} is not a valid utf-8 path name", path.display()))?
            .to_owned();

        // Extract the content type
        let content_type = tree_magic::from_filepath(&path);

        Ok(Asset {
            path,
            name,
            content_type,
        })
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn content_type(&self) -> &str {
        &self.content_type
    }
}

pub fn user_repo_from_url(url: &str) -> Result<(String, String), failure::Error> {
    let path = match Url::parse(url) {
        Err(ParseError::RelativeUrlWithoutBase) => match url.rfind(':') {
            None => return Err(failure::err_msg("Can't parse path from remote URL")),
            Some(colon_pos) => Some(
                url[colon_pos + 1..]
                    .split('/')
                    .map(|s| s.to_owned())
                    .collect::<Vec<_>>(),
            ),
        },
        Err(_) => return Err(failure::err_msg("Can't parse remote URL")),
        Ok(url) => url
            .path_segments()
            .map(|path| path.map(|seg| seg.to_owned()).collect::<Vec<_>>()),
    };

    let path = match path {
        Some(ref path) if path.len() == 2 => path,
        _ => return Err(failure::err_msg("Remote URL should contain user and repository")),
    };

    let user = path[0].clone();
    let repo = match path[1].rfind(".git") {
        None => path[1].clone(),
        Some(suffix_pos) => {
            let valid_pos = path[1].len() - 4;
            if valid_pos == suffix_pos {
                let path = &path[1][0..suffix_pos];
                path.into()
            } else {
                path[1].clone()
            }
        }
    };

    Ok((user, repo))
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn parses_remote_urls() {
        let urls = [
            "https://github.com/user/repo.git",
            "https://github.com/user/repo",
            "git@github.com:user/repo.git",
            "git@github.com:user/repo",
            "ssh://github.com/user/repo",
            "ssh://github.com/user/repo.git",
        ];

        for url in &urls {
            println!("Testing '{:?}'", url);
            let (user, repo) = user_repo_from_url(url).unwrap();

            assert_eq!("user", user);
            assert_eq!("repo", repo);
        }
    }

    #[test]
    fn parses_other_urls() {
        let urls = [("https://github.com/user/repo.git.repo", "user", "repo.git.repo")];

        for &(url, exp_user, exp_repo) in &urls {
            println!("Testing '{:?}'", url);
            let (user, repo) = user_repo_from_url(url).unwrap();

            assert_eq!(exp_user, user);
            assert_eq!(exp_repo, repo);
        }
    }

    #[test]
    fn fail_some_urls() {
        let urls = [
            "https://github.com/user",
            "https://github.com/user/repo/issues",
            "://github.com/user/",
        ];

        for url in &urls {
            println!("Testing '{:?}'", url);
            assert!(user_repo_from_url(url).is_err());
        }
    }
}
