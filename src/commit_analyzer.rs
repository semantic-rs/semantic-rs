use clog::Clog;
use clog::error::Error;

#[derive(PartialEq,Eq,Debug,PartialOrd,Ord)]
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

    if commit.breaks.len() > 0 {
        return Ok(Major);
    }

    let commit_type = match &commit.commit_type[..] {
        "Features" => Minor,
        "Bug Fixes" => Patch,
        _ => Unknown,
    };

    Ok(commit_type)
}

pub fn analyze(commits: &[&str]) -> Result<CommitType, Error> {
    let commit_type = commits.into_iter()
        .map(|commit| analyze_single(commit).expect("Can't analyze commit"))
        .map(|commit| {
            commit
        })
    .max();

    Ok(commit_type.unwrap_or(Unknown))
}

#[test]
fn unknown_type() {
    let commits = vec!["0\nThis commit message has no type"];
    assert_eq!(Unknown, analyze(&commits).unwrap());
}

#[test]
fn patch_commit() {
    let commits = vec!["0\nfix: This commit fixes a bug"];
    assert_eq!(Patch, analyze(&commits).unwrap());
    assert_eq!(Patch, analyze_single(commits[0]).unwrap());
}

#[test]
fn minor_commit() {
    let commits = vec!["0\nfeat: This commit introduces a new feature"];
    assert_eq!(Minor, analyze(&commits).unwrap());
}

#[test]
fn major_commit() {
    let commits = vec!["0\nfeat: This commits breaks something\nBREAKING CHANGE: breaks things"];
    assert_eq!(Major, analyze(&commits).unwrap());
}

#[test]
fn major_commit_multiple() {
    let commits = vec!["0\nfeat: This commits breaks something\n\nBREAKING CHANGE: breaks things",
    "0\nfix: Simple fix"];
    assert_eq!(Major, analyze(&commits).unwrap());
}

#[test]
fn minor_commit_multiple() {
    let commits = vec!["0\nfeat: This commits introduces a new feature", "fix: Simple fix"];
    assert_eq!(Minor, analyze(&commits).unwrap());
}

