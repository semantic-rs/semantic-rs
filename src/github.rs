use hubcaps::releases::{Release, ReleaseOptions};
use hubcaps::{Credentials, Github};
use hyper::net::HttpsConnector;
use hyper::Client;
use hyper_native_tls::NativeTlsClient;
use std::path::Path;

use crate::config::Config;
use crate::error::Error;
use crate::error::Error::GitHub;
use crate::USERAGENT;

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

    releases.create(&opts).map_err(Error::from).map(|release| {
        upload_release_assets(config, release).unwrap_or_else(|e| log::error!("{}", e))
    })
}

fn upload_release_assets(config: &Config, release: Release) -> Result<(), Error> {
    use github_rs::client::{Executor, Github as GH};
    use http::header::{self, HeaderValue};

    let user = &config.user.as_ref().unwrap()[..];
    let repo_name = &config.repository_name.as_ref().unwrap()[..];
    let token = config.gh_token.as_ref().unwrap();

    let gh = GH::new(token)?;

    let mut errored = true;

    for asset in &config.assets {
        let endpoint = format!(
            "https://uploads.github.com/repos/{}/{}/releases/{}/assets?name={}",
            user,
            repo_name,
            release.id,
            asset.name(),
        );

        let body = read_file(asset.path())?;

        let (_, status, response) = gh
            .post(body)
            .custom_endpoint(&endpoint)
            .set_header(
                header::CONTENT_TYPE,
                HeaderValue::from_str(asset.content_type())
                    .expect("failed to construct content type header from content type mime"),
            )
            .execute::<serde_json::Value>()?;

        if !status.is_success() {
            log::error!("failed to upload asset {}", asset.name());
            log::error!("GitHub response: {:#?}", response);
            errored = true;
        }
    }

    if errored {
        Err(Error::Custom("failed to upload some assets".into()))
    } else {
        Ok(())
    }
}

fn read_file(path: impl AsRef<Path>) -> Result<Vec<u8>, Error> {
    Ok(std::fs::read(path)?)
}
