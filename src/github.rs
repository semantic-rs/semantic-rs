use hyper::Client;
use hyper::net::HttpsConnector;
use hyper_native_tls::NativeTlsClient;
use hubcaps::{Github, Credentials};
use hubcaps::releases::ReleaseOptions;

use crate::error::Error;
use crate::USERAGENT;
use crate::config::Config;

pub fn can_release(config: &Config) -> bool {
    let repo = &config.repository;
    match repo.find_remote("origin") {
        Ok(remote) => {
            let url = match remote.url() {
                Some(u) => u,
                None => return false
            };
            is_github_url(url)
        },
        Err(_) => false
    }
}

pub fn is_github_url(url: &str) -> bool {
    url.contains("github.com")
}

pub fn release(config: &Config, tag_name: &str, tag_message: &str) -> Result<(), Error> {
    let user      = &config.user.as_ref().unwrap()[..];
    let repo_name = &config.repository_name.as_ref().unwrap()[..];
    let branch    = &config.branch[..];
    let token     = config.gh_token.as_ref().unwrap();

    let client = Client::with_connector(
        HttpsConnector::new(
            NativeTlsClient::new().unwrap()
        )
    );
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
    let release = repo.releases();

    release
        .create(&opts)
        .map(|_| ())
        .map_err(Error::from)
}
