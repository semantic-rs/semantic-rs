use std::ops::Try;

use clog::Clog;

use crate::config::CfgMapExt;
use crate::plugin::proto::{
    request,
    response::{self, PluginResponse},
    GitRevision, Version,
};
use crate::plugin::{PluginInterface, PluginStep};

pub struct ClogPlugin {}

impl ClogPlugin {
    pub fn new() -> Self {
        ClogPlugin {}
    }
}

impl PluginInterface for ClogPlugin {
    fn methods(&self, _req: request::Methods) -> response::Methods {
        let methods = vec![PluginStep::DeriveNextVersion, PluginStep::GenerateNotes];
        PluginResponse::from_ok(methods)
    }

    fn derive_next_version(
        &self,
        params: request::DeriveNextVersion,
    ) -> response::DeriveNextVersion {
        let (cfg, current_version) = (params.cfg_map, params.data);

        let bump = match &current_version {
            Version::None(_) => CommitType::Major,
            Version::Semver(rev, _) => version_bump_since_rev(&cfg.project_root()?, &rev)?,
        };

        let next_version = match current_version {
            Version::None(_) => semver::Version::new(0, 1, 0),
            Version::Semver(_, mut version) => {
                // NB: According to the Semver spec, major version zero is for
                // the initial development phase is treated slightly differently.
                // The minor version is incremented for breaking changes
                // and major is kept at zero until the public API has become more stable.
                if version.major == 0 {
                    match bump {
                        CommitType::Unknown => (),
                        CommitType::Patch => version.increment_patch(),
                        CommitType::Minor => version.increment_patch(),
                        CommitType::Major => version.increment_minor(),
                    }
                } else {
                    match bump {
                        CommitType::Unknown => (),
                        CommitType::Patch => version.increment_patch(),
                        CommitType::Minor => version.increment_minor(),
                        CommitType::Major => version.increment_major(),
                    }
                }

                version
            }
        };

        PluginResponse::from_ok(next_version)
    }

    fn generate_notes(&self, params: request::GenerateNotes) -> response::GenerateNotes {
        let (cfg, data) = (params.cfg_map, params.data);

        let changelog =
            generate_changelog(&cfg.project_root()?, &data.start_rev, &data.new_version)?;

        PluginResponse::from_ok(changelog)
    }
}

fn version_bump_since_rev(path: &str, rev: &GitRevision) -> Result<CommitType, failure::Error> {
    let repo = Repository::open(path)?;
    let range = format!("{}..HEAD", rev);
    log::debug!("analyzing commits {} to determine version bump", range);

    let mut walker = repo.revwalk()?;
    walker.push_range(&range)?;

    let bump = walker
        .map(|c| {
            repo.find_commit(c.expect("not a valid commit"))
                .expect("no commit found")
        })
        .map(format_commit)
        .map(|c| analyze_single(&c).expect("commit analysis failed"))
        .max()
        .unwrap_or(CommitType::Unknown);

    Ok(bump)
}

fn format_commit(commit: Commit) -> String {
    format!("{}\n{}", commit.id(), commit.message().unwrap_or(""))
}

#[derive(PartialEq, Eq, Debug, PartialOrd, Ord)]
pub enum CommitType {
    Unknown,
    Patch,
    Minor,
    Major,
}

use self::CommitType::*;
use clog::fmt::MarkdownWriter;
use git2::{Commit, Repository};
use std::io::BufWriter;

pub fn analyze_single(commit_str: &str) -> Result<CommitType, failure::Error> {
    let message = commit_str.trim().split_terminator('\n').nth(1);

    let clog = Clog::new().expect("Clog initialization failed");
    let commit = clog.parse_raw_commit(commit_str);

    if !commit.breaks.is_empty() {
        return Ok(Major);
    }

    let commit_type = match &commit.commit_type[..] {
        "Features" => Minor,
        "Bug Fixes" => Patch,
        _ => Unknown,
    };

    if let Some(message) = message {
        log::debug!("derived commit type {:?} for {}", commit_type, message);
    }

    Ok(commit_type)
}

pub fn generate_changelog(
    repository_path: &str,
    from_rev: &str,
    new_version: &semver::Version,
) -> Result<String, failure::Error> {
    log::debug!("generating changelog {}..{}", from_rev, new_version);

    let mut clog = Clog::with_dir(repository_path)?;

    clog.from(from_rev).version(format!("v{}", new_version));

    let mut out_buf = BufWriter::new(Vec::new());

    {
        let mut writer = MarkdownWriter::new(&mut out_buf);
        clog.write_changelog_with(&mut writer)?
    }

    let out_buf = out_buf.into_inner().unwrap();
    let changelog = String::from_utf8(out_buf).unwrap();

    match changelog.find('\n') {
        Some(newline_offset) => Ok(changelog[newline_offset + 1..].into()),
        None => Ok(changelog),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unknown_type() {
        let commit = "0\nThis commit message has no type";
        assert_eq!(CommitType::Unknown, analyze_single(commit).unwrap());
    }

    #[test]
    fn patch_commit() {
        let commit = "0\nfix: This commit fixes a bug";
        assert_eq!(CommitType::Patch, analyze_single(commit).unwrap());
    }

    #[test]
    fn minor_commit() {
        let commit = "0\nfeat: This commit introduces a new feature";
        assert_eq!(CommitType::Minor, analyze_single(commit).unwrap());
    }

    #[test]
    fn major_commit() {
        let commit = "0\nfeat: This commits breaks something\nBREAKING CHANGE: breaks things";
        assert_eq!(CommitType::Major, analyze_single(commit).unwrap());
    }
}
