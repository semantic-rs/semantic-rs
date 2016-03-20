#![cfg_attr(feature = "dev", allow(unstable_features))]
#![cfg_attr(feature = "dev", feature(plugin))]
#![cfg_attr(feature = "dev", plugin(clippy))]

mod logger;
mod toml_file;
mod git;
mod changelog;
mod commit_analyzer;
mod cargo;
mod error;
mod github;
mod config;

extern crate rustc_serialize;
extern crate toml;
extern crate regex;
extern crate semver;
extern crate docopt;
extern crate git2;
extern crate clog;
extern crate hyper;
extern crate hubcaps;
extern crate url;

use docopt::Docopt;
use commit_analyzer::CommitType;
use config::ConfigBuilder;
use std::process;
use semver::Version;
use std::{env,fs};
use std::path::Path;
use std::error::Error;
use url::Url;

const VERSION: &'static str = env!("CARGO_PKG_VERSION");
const USERAGENT: &'static str = concat!("semantic-rs/", env!("CARGO_PKG_VERSION"));
const USAGE: &'static str = "
semantic.rs 🚀

Usage:
  semantic-rs [options]
  semantic-rs --version

Options:
  -h --help              Show this screen.
  --version              Show version.
  -p PATH, --path=PATH   Specifies the repository path. [default: .]
  -w, --write            Run with writing the changes afterwards.
";

macro_rules! print_exit {
    ($fmt:expr) => {{
        logger::stderr(format!($fmt));
        process::exit(1);
    }};
    ($fmt:expr, $($arg:tt)*) => {{
        logger::stderr(format!($fmt, $($arg)*));
        process::exit(1);
    }};
}

#[derive(Debug, RustcDecodable)]
struct Args {
    flag_path: String,
    flag_write: bool,
    flag_version: bool,
}

fn version_bump(version: &Version, bump: CommitType) -> Option<Version> {
    let mut version = version.clone();
    match bump {
        CommitType::Unknown => return None,
        CommitType::Patch => version.increment_patch(),
        CommitType::Minor => version.increment_minor(),
        CommitType::Major => version.increment_major(),
    }

    Some(version)
}

fn ci_env_set() -> bool {
    env::var("CI").is_ok()
}

fn user_repo_from_url(url: Url) -> Result<(String, String), String> {
    let path = match url.path() {
        Some(path) => path,
        None => return Err("URL should contain user and repository".into()),
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
                return Err(".git suffix not at the end of URL".into())
            }
        }
    };

    Ok((user, repo))
}

fn main() {
    let args: Args = Docopt::new(USAGE)
        .and_then(|d| d.decode())
        .unwrap_or_else(|e| e.exit());

    let mut cb = ConfigBuilder::new();

    if args.flag_version {
        println!("semantic.rs 🚀 -- v{}", VERSION);
        process::exit(0);
    }

    let is_dry_run = if ci_env_set() {
        false
    }
    else {
        !args.flag_write
    };

    cb.write(args.flag_write);

    println!("semantic.rs 🚀");

    logger::stdout("Analyzing your repository");
    let path = Path::new(&args.flag_path);
    let path = fs::canonicalize(path)
        .unwrap_or_else(|_| print_exit!("Path does not exist or a component is not a directory"));
    let repository_path = path.to_str()
        .unwrap_or_else(|| print_exit!("Path is not valid unicode"));

    let repo = match git2::Repository::open(repository_path) {
        Ok(repo) => repo,
        Err(e) => {
            logger::stderr(format!("Could not open the git repository: {:?}", e));
            process::exit(1);
        }
    };

    cb.repository_path(repository_path.to_owned());

    // extra scope scope to make sure borrow of `repo` is dropped
    {
        let signature = match git::get_signature(&repo) {
            Ok(sig) => sig,
            Err(e) => {
                logger::stderr(format!("Failed to get the committer's name and email address: {}", e.description()));
                logger::stderr(r"
A release commit needs a committer name and email address.
We tried fetching it from different locations, but couldn't find one.

Committer information is taken from the following environment variables, if set:

GIT_COMMITTER_NAME
GIT_COMMITTER_EMAIL

If none is set the normal git config is tried in the following order:

Local repository config
User config
Global config");
                process::exit(1);
            }
        };

        cb.signature(signature.to_owned());
    }

    let remote_url = match repo.find_remote("origin") {
        Err(e) => print_exit!("Could not determine the origin remote url: {:?}", e),
        Ok(remote) => {
            let url = remote.url().expect("Remote URL is not valid UTF-8");
            Url::parse(&url).expect("Remote URL can't be parsed")
        }
    };

    let (user, repo_name) = user_repo_from_url(remote_url)
        .unwrap_or_else(|e| print_exit!("Could not extract user and repository name from URL: {:?}", e));
    cb.user(user);
    cb.repository_name(repo_name);

    let gh_token = env::var("GH_TOKEN")
        .unwrap_or_else(|err| print_exit!("GH_TOKEN not set: {:?}", err));

    let travis_token = env::var("TRAVIS_TOKEN")
        .unwrap_or_else(|err| print_exit!("TRAVIS_TOKEN not set: {:?}", err));

    cb.gh_token(gh_token);
    cb.travis_token(travis_token);

    cb.repository(repo);
    cb.branch("master".into());
    let config = cb.build();

    let version = toml_file::read_from_file(&config.repository_path)
        .unwrap_or_else(|err| print_exit!("Reading `Cargo.toml` failed: {:?}", err));

    let version = Version::parse(&version).expect("Not a valid version");
    logger::stdout(format!("Current version: {}", version.to_string()));

    logger::stdout("Analyzing commits");

    let bump = git::version_bump_since_latest(&config.repository);
    if is_dry_run {
        logger::stdout(format!("Commits analyzed. Bump would be {:?}", bump));
    }
    else {
        logger::stdout(format!("Commits analyzed. Bump will be {:?}", bump));
    }
    let new_version = match version_bump(&version, bump) {
        Some(new_version) => new_version.to_string(),
        None => {
            logger::stdout("No version bump. Nothing to do.");
            process::exit(0);
        }
    };

    if is_dry_run {
        logger::stdout(format!("New version would be: {}", new_version));
        logger::stdout("Would write the following Changelog:");
        let changelog = match changelog::generate(repository_path, &version.to_string(), &new_version) {
            Ok(log) => log,
            Err(err) => {
                logger::stderr(format!("Generating Changelog failed: {:?}", err));
                process::exit(1);
            }
        };
        logger::stdout("====================================");
        logger::stdout(changelog);
        logger::stdout("====================================");
        logger::stdout("Would create annotated git tag");
    }
    else {
        logger::stdout(format!("New version: {}", new_version));

        toml_file::write_new_version(repository_path, &new_version)
            .unwrap_or_else(|err| print_exit!("Writing `Cargo.toml` failed: {:?}", err));

        logger::stdout(format!("Writing Changelog"));
        changelog::write(repository_path, &version.to_string(), &new_version)
            .unwrap_or_else(|err| print_exit!("Writing Changelog failed: {:?}", err));

        logger::stdout("Updating lockfile");
        if !cargo::update_lockfile(repository_path) {
            print_exit!("`cargo fetch` failed. See above for the cargo error message.");
        }

        logger::stdout("Package crate");
        if !cargo::package(repository_path) {
            print_exit!("`cargo package` failed. See above for the cargo error message.");
        }

        git::commit_files(&config.repository, &new_version)
            .unwrap_or_else(|err| print_exit!("Committing files failed: {:?}", err));

        logger::stdout("Creating annotated git tag");
        let tag_message = changelog::generate(repository_path, &version.to_string(), &new_version)
            .unwrap_or_else(|err| print_exit!("Can't generate changelog: {:?}", err));

        let tag_name = format!("v{}", new_version);
        git::tag(&config.repository, &tag_name, &tag_message)
            .unwrap_or_else(|err| print_exit!("Failed to create git tag: {:?}", err));

        logger::stdout("Pushing new commit and tag");
        git::push(&config, &tag_name)
            .unwrap_or_else(|err| print_exit!("Failed to push git: {:?}", err));

        logger::stdout("Creating GitHub release");
        github::release(&config, &tag_name, &tag_message)
            .unwrap_or_else(|err| print_exit!("Failed to create GitHub release: {:?}", err));

        logger::stdout("Publishing crate on crates.io");
        if !cargo::publish(&config.repository_path, &config.travis_token.as_ref().unwrap()) {
            print_exit!("Failed to publish on crates.io");
        }
    }
}
