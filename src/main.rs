mod git_history;
mod git_client;
mod logger;
mod toml_file;
extern crate toml;
extern crate regex;
extern crate semver;
extern crate commit_walker;
extern crate commit_analyzer;

use commit_analyzer::CommitType;

fn version_bump(version: Version, bump: CommitType) -> Option<Version> {
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

fn print_log(log_entries: Vec<git_history::LogEntry>) {
    for entry in log_entries {
        logger::stdout(entry.revision);
        logger::stdout(entry.title);
    }
}

fn main() {
    println!("semantic.rs 🚀");

    logger::stdout("Analyzing your repository");

    let version = match toml_file::read_from_file() {
        Ok(toml) => toml,
        Err(e) => {
            logger::stderr(format!("Reading `Cargo.toml` failed: {:?}", e));
            process::exit(1);
        }
    };

    let version = Version::parse(&version).expect("Not a valid version");
    logger::stdout(format!("Current version: {}", version.to_string()));

    logger::stdout("Analyzing commits");

    let bump = commit_walker::version_bump_since_latest(".");
    logger::stdout(format!("Commits analyzed. Bump will be {:?}", bump));

    let new_version = match version_bump(version, bump) {
        Some(new_version) => new_version,
        None => {
            logger::stdout("No version bump. Nothing to do.");
            process::exit(0);
        }
    };

    logger::stdout(format!("New version: {}", new_version.to_string()));
    toml_file::write_new_version(new_version.to_string());
}
