use std::ops::Try;
use std::path::{Path, PathBuf};

use failure::Error;
use failure::Fail;
use http::header::HeaderValue;
use hubcaps::releases::ReleaseOptions;
use hubcaps::{Credentials, Github};
use hyper::net::HttpsConnector;
use hyper::Client;
use hyper_native_tls::NativeTlsClient;
use serde::Deserialize;
use url::{ParseError, Url};

use crate::config::CfgMapExt;
use crate::plugin::proto::{
    request,
    response::{self, PluginResponse},
};
use crate::plugin::{PluginInterface, PluginStep};
use crate::utils::ResultExt;

const USERAGENT: &str = concat!("semantic-rs/", env!("CARGO_PKG_VERSION"));

pub struct GithubPlugin {}

impl GithubPlugin {
    pub fn new() -> Self {
        GithubPlugin {}
    }
}

#[derive(Deserialize)]
pub struct GithubPluginConfig {
    assets: Vec<String>,
    user: Option<String>,
    repository: Option<String>,
    #[serde(default = "default_remote")]
    remote: String,
    #[serde(default = "default_branch")]
    branch: String,
}

fn default_remote() -> String {
    "origin".into()
}

fn default_branch() -> String {
    "master".into()
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

    fn methods(&self, _req: request::Methods) -> response::Methods {
        let methods = vec![PluginStep::PreFlight, PluginStep::Publish];
        PluginResponse::from_ok(methods)
    }

    fn pre_flight(&mut self, params: request::PreFlight) -> response::PreFlight {
        let mut response = PluginResponse::builder();

        if !params.env.contains_key("GH_TOKEN") {
            response.error(GithubPluginError::TokenUndefined);
        }

        // Try to parse config
        let cfg: GithubPluginConfig =
            toml::Value::Table(params.cfg_map.get_sub_table("github")?).try_into()?;

        // Try to parse assets
        globs_to_assets(cfg.assets.iter().map(String::as_str))
            .into_iter()
            .filter(Result::is_err)
            .map(Result::unwrap_err)
            .for_each(|e| {
                response.error(e);
            });

        response.body(()).build()
    }

    fn publish(&mut self, params: request::Publish) -> response::Publish {
        let cfg: GithubPluginConfig =
            toml::Value::Table(params.cfg_map.get_sub_table("github")?).try_into()?;
        let project_root = Path::new(params.cfg_map.project_root()?);

        let repo = git2::Repository::open(project_root)?;
        let remote = repo.find_remote(&cfg.remote)?;
        let remote_url = remote.url().ok_or(GithubPluginError::GitRemoteUndefined)?;

        let (derived_name, derived_repo) = user_repo_from_url(remote_url)?;

        let user = cfg.user.as_ref().unwrap_or(&derived_name);
        let repo_name = cfg.repository.as_ref().unwrap_or(&derived_repo);
        let branch = &cfg.branch;
        let tag_name = &params.data.tag_name;
        let changelog = &params.data.changelog;
        let token = std::env::var("GH_TOKEN").map_err(|_| GithubPluginError::TokenUndefined)?;

        // Create release
        let client = Client::with_connector(HttpsConnector::new(NativeTlsClient::new().unwrap()));
        let credentials = Credentials::Token(token.to_owned());
        let github = Github::new(USERAGENT, client, credentials);

        let opts = ReleaseOptions::builder(tag_name)
            .name(tag_name)
            .body(changelog)
            .commitish(branch)
            .draft(false)
            .prerelease(false)
            .build();

        let repo = github.repo(user, repo_name);
        let releases = repo.releases();

        let release = releases.create(&opts).sync()?;

        // Upload assets
        let token_header_value = HeaderValue::from_str(&format!("token {}", token)).unwrap();

        let mut errored = false;

        let assets = globs_to_assets(cfg.assets.iter().map(String::as_str))
            .into_iter()
            .collect::<Result<Vec<_>, _>>()?;

        for asset in assets {
            let endpoint = format!(
                "https://uploads.github.com/repos/{}/{}/releases/{}/assets?name={}",
                user,
                repo_name,
                release.id,
                asset.name(),
            );

            log::info!(
                "Uploading {}, mime-type {}",
                asset.name(),
                asset.content_type()
            );
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
            Err(failure::err_msg("failed to upload some assets"))?;
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
            return Err(failure::format_err!(
                "asset file not found at {}",
                path.display()
            ));
        }

        // Check is asset is file
        if !path.is_file() {
            return Err(failure::format_err!(
                "asset at {} is not a file",
                path.display()
            ));
        }

        // Create a name from the file path
        let name = path
            .file_name()
            .ok_or_else(|| failure::format_err!("couldn't get a file stem for {}", path.display()))?
            .to_str()
            .ok_or_else(|| {
                failure::format_err!("{} is not a valid utf-8 path name", path.display())
            })?
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
        _ => {
            return Err(failure::err_msg(
                "Remote URL should contain user and repository",
            ))
        }
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
        let urls = [(
            "https://github.com/user/repo.git.repo",
            "user",
            "repo.git.repo",
        )];

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

#[derive(Fail, Debug)]
pub enum GithubPluginError {
    #[fail(display = "the GH_TOKEN environment variable is not configured")]
    TokenUndefined,
    #[fail(display = "failed to determine git remote url")]
    GitRemoteUndefined,
}
