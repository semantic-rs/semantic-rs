use std::env;
use hyper::Client;
use hubcaps::{Github, ReleaseOptions};
use error::Error;
use super::USERAGENT;

pub fn release(tag_name: &str, tag_message: &str) -> Result<(), Error> {
    let token = try!(env::var("GITHUB_TOKEN"));

    let client = Client::new();
    let github = Github::new(USERAGENT, &client, Some(token));

    let user = "badboy";
    let repo = "test-project";
    let branch = "master"; // TODO: Extract from environment, might be != master

    let opts = ReleaseOptions::builder(tag_name)
        .name(tag_name)
        .body(tag_message)
        .commitish(branch)
        .draft(false)
        .prerelease(false)
        .build();

    let repo = github.repo(user, repo);
    let release = repo.releases();

    release
        .create(&opts)
        .map(|_| ())
        .map_err(Error::from)
}
