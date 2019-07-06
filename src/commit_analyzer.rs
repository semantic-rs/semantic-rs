use clog::error::Error;
use clog::Clog;

#[derive(PartialEq, Eq, Debug, PartialOrd, Ord)]
pub enum CommitType {
    Unknown,
    Patch,
    Minor,
    Major,
}

use self::CommitType::*;

pub fn analyze_single(commit: &str) -> Result<CommitType, Error> {
    let clog = Clog::new().expect("Clog initialization failed");
    let commit = clog.parse_raw_commit(commit);

    if !commit.breaks.is_empty() {
        return Ok(Major);
    }

    let commit_type = match &commit.commit_type[..] {
        "Features" => Minor,
        "Bug Fixes" => Patch,
        _ => Unknown,
    };

    Ok(commit_type)
}

#[test]
fn unknown_type() {
    let commit = "0\nThis commit message has no type";
    assert_eq!(Unknown, analyze_single(commit).unwrap());
}

#[test]
fn patch_commit() {
    let commit = "0\nfix: This commit fixes a bug";
    assert_eq!(Patch, analyze_single(commit).unwrap());
}

#[test]
fn minor_commit() {
    let commit = "0\nfeat: This commit introduces a new feature";
    assert_eq!(Minor, analyze_single(commit).unwrap());
}

#[test]
fn major_commit() {
    let commit = "0\nfeat: This commits breaks something\nBREAKING CHANGE: breaks things";
    assert_eq!(Major, analyze_single(commit).unwrap());
}
