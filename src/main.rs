mod logger;
mod toml_file;
mod git;
mod changelog;
mod commit_analyzer;

extern crate rustc_serialize;
extern crate toml;
extern crate regex;
extern crate semver;
extern crate docopt;
extern crate git2_commit;
extern crate git2;
extern crate clog;

use docopt::Docopt;
use commit_analyzer::CommitType;
use std::process;
use semver::Version;

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

fn main() {
    let args: Args = Docopt::new(USAGE)
        .and_then(|d| d.decode())
        .unwrap_or_else(|e| e.exit());

    if args.flag_version {
        println!("semantic.rs 🚀 -- v{}", VERSION);
        process::exit(0);
    }

    let is_dry_run = !args.flag_write;

    println!("semantic.rs 🚀");

    logger::stdout("Analyzing your repository");
    let repository_path = &args.flag_path;

    match git2::Repository::open(repository_path) {
        Ok(_) => { },
        Err(e) => {
            logger::stderr(format!("Could not open the git repository: {:?}", e));
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
        let changelog = match changelog::generate(repository_path, &version.to_string(), &new_version.to_string()) {
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

        logger::stdout(format!("Writing Changelog"));
        match changelog::write(repository_path, &version.to_string(), &new_version.to_string()) {
            Ok(_)    => { },
            Err(err) => {
                logger::stderr(format!("Writing Changelog failed: {:?}", err));
                process::exit(1);
            }
        }

        match git::commit_files(repository_path, &new_version) {
            Ok(_)    => { },
            Err(err) => {
                logger::stderr(format!("Committing `Cargo.toml` and `Changelog.md` failed: {:?}", err));
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
