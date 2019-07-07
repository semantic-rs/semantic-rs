use crate::config::Config;
use crate::utils::ResultExt;
use crate::USERAGENT;
use failure::Error;
use http::header::HeaderValue;
use hubcaps::releases::{Release, ReleaseOptions};
use hubcaps::{Credentials, Github};
use hyper::net::HttpsConnector;
use hyper::Client;
use hyper_native_tls::NativeTlsClient;
use std::path::Path;

pub fn can_release(config: &Config) -> bool {
    let repo = &config.repository;
    match repo.find_remote("origin") {
        Ok(remote) => {
            let url = match remote.url() {
                Some(u) => u,
                None => return false,
            };
            is_github_url(url)
        }
        Err(_) => false,
    }
}

pub fn is_github_url(url: &str) -> bool {
    url.contains("github.com")
}

pub fn release(config: &Config, tag_name: &str, tag_message: &str) -> Result<(), Error> {
    let user = &config.user.as_ref().unwrap()[..];
    let repo_name = &config.repository_name.as_ref().unwrap()[..];
    let branch = &config.branch[..];
    let token = config.gh_token.as_ref().unwrap();

    let client = Client::with_connector(HttpsConnector::new(NativeTlsClient::new().unwrap()));
    let credentials = Credentials::Token(token.to_owned());
    let github = Github::new(USERAGENT, client, credentials);

    let opts = ReleaseOptions::builder(tag_name)
        .name(tag_name)
        .body(tag_message)
        .commitish(branch)
        .draft(false)
        .prerelease(false)
        .build();

    let repo = github.repo(user, repo_name);
    let releases = repo.releases();

    let release = releases.create(&opts).sync()?;

    upload_release_assets(config, release).unwrap_or_else(|e| log::error!("{}", e));

    Ok(())
}

fn upload_release_assets(config: &Config, release: Release) -> Result<(), Error> {
    let user = &config.user.as_ref().unwrap()[..];
    let repo_name = &config.repository_name.as_ref().unwrap()[..];
    let token = config.gh_token.as_ref().unwrap();
    let token_header_value = HeaderValue::from_str(&format!("token {}", token)).unwrap();

    let mut errored = false;

    for asset in &config.assets {
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

        let body = read_file(asset.path())?;

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
        Err(failure::err_msg("failed to upload some assets"))
    } else {
        Ok(())
    }
}

fn read_file(path: impl AsRef<Path>) -> Result<Vec<u8>, Error> {
    Ok(std::fs::read(path)?)
}
