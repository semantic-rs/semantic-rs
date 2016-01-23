mod logger;
mod toml_file;
mod git;
mod changelog;
mod commit_analyzer;
extern crate toml;
extern crate regex;
extern crate semver;
extern crate argparse;
extern crate git2_commit;
extern crate git2;
extern crate time;
extern crate clog;

use argparse::{ArgumentParser, StoreTrue, Store};
use commit_analyzer::CommitType;

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

use std::process;
use semver::Version;

fn get_repository_path() -> String {
    let mut path = ".".to_string();
    {
        let mut ap = ArgumentParser::new();
        ap.refer(&mut path)
            .add_option(&["-p", "--path"], Store,
                        "Specifies the repository path. If ommitted it defaults to current directory");
        ap.parse_args_or_exit();
    }
    path
}

fn main() {
    println!("semantic.rs 🚀");

    logger::stdout("Analyzing your repository");
    let repository_path = get_repository_path();

    match git2::Repository::open(&repository_path) {
        Ok(_) => { },
        Err(e) => {
            logger::stderr(format!("Could not open the git repository: {:?}", e));
            process::exit(1);
        }
    };

    let version = match toml_file::read_from_file(&repository_path) {
        Ok(toml) => toml,
        Err(e) => {
            logger::stderr(format!("Reading `Cargo.toml` failed: {:?}", e));
            process::exit(1);
        }
    };

    let version = Version::parse(&version).expect("Not a valid version");
    logger::stdout(format!("Current version: {}", version.to_string()));

    logger::stdout("Analyzing commits");

    let bump = git::version_bump_since_latest(&repository_path);
    logger::stdout(format!("Commits analyzed. Bump will be {:?}", bump));

    let new_version = match version_bump(&version, bump) {
        Some(new_version) => new_version,
        None => {
            logger::stdout("No version bump. Nothing to do.");
            process::exit(0);
        }
    };

    logger::stdout(format!("New version: {}", new_version.to_string()));
    match toml_file::write_new_version(&repository_path, new_version.to_string()) {
        Ok(_)    => { },
        Err(err) => {
            logger::stderr(format!("Writing `Cargo.toml` failed: {:?}", err));
            process::exit(1);
        }
    }

    logger::stdout(format!("Writing Changelog"));
    match changelog::write(&repository_path, &version.to_string(), &new_version.to_string()) {
        Ok(_)    => { },
        Err(err) => {
            logger::stderr(format!("Writing Changelog failed: {:?}", err));
            process::exit(1);
        }
    }

    match git::commit_files(&repository_path, new_version.to_string()) {
        Ok(_)    => { },
        Err(err) => {
            logger::stderr(format!("Committing `Cargo.toml` failed: {:?}", err));
            process::exit(1);
        }
    }

    logger::stdout("Creating annotated git tag");
    match git::tag(&repository_path, &new_version.to_string()) {
        Ok(_) => { },
        Err(err) => logger::stderr(format!("Failed to create git tag: {:?}", err))
    }
}
