use git2::{self, Commit, Cred, PushOptions, RemoteCallbacks, Repository, Signature};
use semver::Version;
use std::env;
use std::path::Path;

use crate::commit_analyzer::{self, CommitType};
use crate::config::Config;
use crate::error::Error;

pub fn get_signature(repo: &Repository) -> Result<Signature, Error> {
    let author = {
        let mut author = env::var("GIT_COMMITTER_NAME").map_err(Error::from);

        if author.is_err() {
            let config = repo.config()?;
            author = config.get_string("user.name").map_err(Error::from);
        }

        author?
    };

    let email = {
        let mut email = env::var("GIT_COMMITTER_EMAIL").map_err(Error::from);

        if email.is_err() {
            let config = repo.config()?;
            email = config.get_string("user.email").map_err(Error::from);
        }

        email?
    };

    Signature::now(&author, &email).map_err(From::from)
}

fn range_to_head(commit: &str) -> String {
    format!("{}..HEAD", commit)
}

fn format_commit(commit: Commit) -> String {
    format!("{}\n{}", commit.id(), commit.message().unwrap_or(""))
}

fn add<P: AsRef<Path>>(repo: &Repository, files: &[P]) -> Result<(), git2::Error> {
    let mut index = repo.index()?;

    for path in files {
        index.add_path(path.as_ref())?;
    }

    index.write()
}

fn commit(config: &Config, message: &str) -> Result<(), git2::Error> {
    let update_ref = format!("refs/heads/{}", config.branch);
    let repo = &config.repository;

    let oid = repo.refname_to_id("HEAD")?;
    let parent_commit = repo.find_commit(oid)?;
    let parents = vec![&parent_commit];

    let mut index = repo.index()?;
    let tree_oid = index.write_tree()?;
    let tree = repo.find_tree(tree_oid)?;

    repo.commit(
        Some(&update_ref),
        &config.signature,
        &config.signature,
        message,
        &tree,
        &parents,
    )
    .map(|_| ())
}

fn create_tag(config: &Config, tag_name: &str, message: &str) -> Result<(), git2::Error> {
    let repo = &config.repository;

    let rev = format!("refs/heads/{}", config.branch);
    let obj = repo.revparse_single(&rev)?;

    repo.tag(tag_name, &obj, &config.signature, message, false)
        .map(|_| ())
}

pub fn is_https_remote(maybe_remote: Option<&str>) -> bool {
    if let Some(remote) = maybe_remote {
        remote.starts_with("https://")
    } else {
        false
    }
}

pub fn latest_tag(repo: &Repository) -> Option<Version> {
    let tags = match repo.tag_names(None) {
        Ok(tags) => tags,
        Err(_) => return None,
    };

    tags.iter()
        .map(|tag| tag.unwrap())
        .filter_map(|tag| Version::parse(&tag[1..]).ok())
        .max()
}

pub fn version_bump_since_latest(repo: &Repository) -> CommitType {
    match latest_tag(repo) {
        Some(t) => {
            let tag = format!("v{}", t.to_string());
            version_bump_since_tag(repo, &tag)
        }
        None => CommitType::Major,
    }
}

pub fn version_bump_since_tag(repo: &Repository, tag: &str) -> CommitType {
    let tag = range_to_head(tag);

    let mut walker = repo.revwalk().expect("Creating a revwalk failed");
    walker.push_range(&tag).expect("Adding a range failed");

    walker
        .map(|c| {
            repo.find_commit(c.expect("Not a valid commit"))
                .expect("No commit found")
        })
        .map(format_commit)
        .map(|c| commit_analyzer::analyze_single(&c).expect("Analyzing commit failed"))
        .max()
        .unwrap_or(CommitType::Unknown)
}

pub fn generate_commit_message(new_version: &str) -> String {
    format!("Bump version to {}", new_version).into()
}

pub fn commit_files(config: &Config, new_version: &str) -> Result<(), Error> {
    let repo = &config.repository;
    let files = ["Cargo.toml", "Cargo.lock", "Changelog.md"];
    let files = files
        .iter()
        .filter(|filename| {
            let path = Path::new(filename);
            !repo
                .status_should_ignore(path)
                .expect("Determining ignore status of file failed")
        })
        .collect::<Vec<_>>();

    add(&config.repository, &files[..])?;

    commit(config, &generate_commit_message(new_version)).map_err(Error::from)
}

pub fn tag(config: &Config, tag_name: &str, tag_message: &str) -> Result<(), Error> {
    create_tag(config, &tag_name, &tag_message).map_err(Error::from)
}

pub fn get_remote_url(config: &Config, remote: &str) -> Result<Option<String>, Error> {
    Ok(config
        .repository
        .find_remote(remote)?
        .url()
        .map(ToOwned::to_owned))
}

pub fn set_remote_url(config: &mut Config, remote: &str, url: &str) -> Result<(), Error> {
    let repo = &mut config.repository;
    repo.remote_set_url(remote, url)?;
    repo.remote_set_pushurl(remote, Some(url))?;
    Ok(())
}

pub fn push(config: &Config, tag_name: &str) -> Result<(), Error> {
    let repo = &config.repository;

    let branch = &config.branch;
    let token = config.gh_token.as_ref();

    // We need to push both the branch we just committed as well as the tag we created.
    let branch_ref = format!("refs/heads/{}", branch);
    let tag_ref = format!("refs/tags/{}", tag_name);
    let refs = [&branch_ref[..], &tag_ref[..]];

    let mut remote = repo.find_remote("origin")?;
    let mut cbs = RemoteCallbacks::new();
    let mut opts = PushOptions::new();

    if is_https_remote(remote.url()) {
        cbs.credentials(|_url, _username, _allowed| Cred::userpass_plaintext(&token.unwrap(), ""));
        opts.remote_callbacks(cbs);
    } else {
        cbs.credentials(|_url, username, _allowed| Cred::ssh_key_from_agent(&username.unwrap()));
        opts.remote_callbacks(cbs);
    }

    remote
        .push(&refs, Some(&mut opts))
        .map(|_| ())
        .map_err(Error::from)
}
