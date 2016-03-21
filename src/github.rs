use hyper::Client;
use hubcaps::{Github, ReleaseOptions};
use error::Error;
use super::USERAGENT;
use config::Config;

pub fn release(config: &Config, tag_name: &str, tag_message: &str) -> Result<(), Error> {
    let user      = &config.user[..];
    let repo_name = &config.repository_name[..];
    let branch    = &config.branch[..];
    let token     = config.gh_token.as_ref().unwrap();

    let client = Client::new();
    let github = Github::new(USERAGENT, &client, Some(&token[..]));

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
