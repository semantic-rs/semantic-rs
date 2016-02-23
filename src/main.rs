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

extern crate rustc_serialize;
extern crate toml;
extern crate regex;
extern crate semver;
extern crate docopt;
extern crate git2;
extern crate clog;

use docopt::Docopt;
use commit_analyzer::CommitType;
use std::process;
use semver::Version;
use std::env;
use std::error::Error;

const VERSION: &'static str = env!("CARGO_PKG_VERSION");
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

fn main() {
    let args: Args = Docopt::new(USAGE)
        .and_then(|d| d.decode())
        .unwrap_or_else(|e| e.exit());

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

    println!("semantic.rs 🚀");

    logger::stdout("Analyzing your repository");
    let repository_path = &args.flag_path;

    let repo = match git2::Repository::open(repository_path) {
        Ok(repo) => repo,
        Err(e) => {
            logger::stderr(format!("Could not open the git repository: {:?}", e));
            process::exit(1);
        }
    };

    let _signature = match git::get_signature(&repo) {
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

    let version = match toml_file::read_from_file(repository_path) {
        Ok(toml) => toml,
        Err(e) => {
            logger::stderr(format!("Reading `Cargo.toml` failed: {:?}", e));
            process::exit(1);
        }
    };

    let version = Version::parse(&version).expect("Not a valid version");
    logger::stdout(format!("Current version: {}", version.to_string()));

    logger::stdout("Analyzing commits");

    let bump = git::version_bump_since_latest(repository_path);
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

        match toml_file::write_new_version(repository_path, &new_version) {
            Ok(_)    => { },
            Err(err) => {
                logger::stderr(format!("Writing `Cargo.toml` failed: {:?}", err));
                process::exit(1);
            }
        }

        logger::stdout("Updating lockfile");
        if !cargo::update_lockfile(repository_path) {
            logger::stderr("`cargo fetch` failed. See above for the cargo error message");
            process::exit(1);
        }

        logger::stdout(format!("Writing Changelog"));
        match changelog::write(repository_path, &version.to_string(), &new_version) {
            Ok(_)    => { },
            Err(err) => {
                logger::stderr(format!("Writing Changelog failed: {:?}", err));
                process::exit(1);
            }
        }

        logger::stdout("Package crate");
        if !cargo::package(repository_path) {
            logger::stderr("`cargo package` failed. See above for the cargo error message");
            process::exit(1);
        }

        match git::commit_files(repository_path, &new_version) {
            Ok(_)    => { },
            Err(err) => {
                logger::stderr(format!("Committing files failed: {:?}", err));
                process::exit(1);
            }
        }

        logger::stdout("Creating annotated git tag");
        let tag_message = match changelog::generate(repository_path, &version.to_string(), &new_version) {
            Ok(msg) => msg,
            Err(err) => {
                logger::stderr(format!("Can't generate changelog: {:?}", err));
                process::exit(1);
            }
        };

        let tag_name = format!("v{}", new_version);
        match git::tag(repository_path, &tag_name, &tag_message) {
            Ok(_) => { },
            Err(err) => {
                logger::stderr(format!("Failed to create git tag: {:?}", err));
                process::exit(1);
            }
        }
    }
}
