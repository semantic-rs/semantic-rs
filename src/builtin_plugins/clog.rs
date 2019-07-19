use std::io::BufWriter;
use std::ops::Try;
use std::path::{Path, PathBuf};

use clog::fmt::MarkdownWriter;
use clog::Clog;
use git2::{Commit, Repository};
use serde::Deserialize;

use crate::config::CfgMapExt;
use crate::plugin::proto::{
    request,
    response::{self, PluginResponse},
    GitRevision,
};
use crate::plugin::{PluginInterface, PluginStep};

pub struct ClogPlugin {
    state: Option<request::GenerateNotesData>,
    dry_run_guard: Option<DryRunGuard>,
}

impl ClogPlugin {
    pub fn new() -> Self {
        ClogPlugin {
            state: None,
            dry_run_guard: None,
        }
    }
}

impl Drop for ClogPlugin {
    fn drop(&mut self) {
        if let Some(guard) = self.dry_run_guard.as_ref() {
            log::info!("clog(dry-run): restoring original state of changelog file");
            if let Err(err) = std::fs::write(&guard.changelog_path, &guard.original_changelog) {
                log::error!("failed to restore original changelog, sorry x_x");
                log::error!("{}", err);
                log::info!(
                    "\nOriginal changelog: \n{}",
                    String::from_utf8_lossy(&guard.original_changelog)
                );
            }
        }
    }
}

struct DryRunGuard {
    changelog_path: PathBuf,
    original_changelog: Vec<u8>,
}

#[derive(Deserialize)]
struct ClogPluginConfig {
    #[serde(default = "default_changelog")]
    changelog: String,
}

fn default_changelog() -> String {
    "Changelog.md".into()
}

impl PluginInterface for ClogPlugin {
    fn name(&self) -> response::Name {
        PluginResponse::from_ok("clog".into())
    }

    fn methods(&self, _req: request::Methods) -> response::Methods {
        let methods = vec![
            PluginStep::PreFlight,
            PluginStep::DeriveNextVersion,
            PluginStep::GenerateNotes,
            PluginStep::Prepare,
        ];
        PluginResponse::from_ok(methods)
    }

    fn pre_flight(&mut self, params: request::PreFlight) -> response::PreFlight {
        // Try to deserialize configuration
        let _: ClogPluginConfig =
            toml::Value::Table(params.cfg_map.get_sub_table("clog")?).try_into()?;
        PluginResponse::from_ok(())
    }

    fn derive_next_version(
        &mut self,
        params: request::DeriveNextVersion,
    ) -> response::DeriveNextVersion {
        let (cfg, current_version) = (params.cfg_map, params.data);

        let bump = match &current_version.semver {
            None => CommitType::Major,
            Some(_) => version_bump_since_rev(&cfg.project_root()?, &current_version.rev)?,
        };

        let next_version = match current_version.semver {
            None => semver::Version::new(0, 1, 0),
            Some(mut version) => {
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

    fn generate_notes(&mut self, params: request::GenerateNotes) -> response::GenerateNotes {
        let (cfg, data) = (params.cfg_map, params.data);

        let changelog =
            generate_changelog(&cfg.project_root()?, &data.start_rev, &data.new_version)?;

        // Store this request as state
        self.state.replace(data);

        PluginResponse::from_ok(changelog)
    }

    fn prepare(&mut self, params: request::Prepare) -> response::Prepare {
        let cfg: ClogPluginConfig =
            toml::Value::Table(params.cfg_map.get_sub_table("clog")?).try_into()?;
        let changelog_path = &cfg.changelog;
        let repo_path = params.cfg_map.project_root()?;

        // Safely store the original changelog for restoration after dry-run is finished
        if params.cfg_map.is_dry_run()? {
            log::info!("clog(dry-run): saving original state of changelog file");
            let original_changelog = std::fs::read(&changelog_path)?;
            self.dry_run_guard.replace(DryRunGuard {
                changelog_path: Path::new(changelog_path).to_owned(),
                original_changelog,
            });
        }

        let state = self
            .state
            .as_ref()
            .expect("state is None: this is a bug, aborting.");

        let mut clog = Clog::with_dir(repo_path)?;
        clog.changelog(changelog_path)
            .from(&state.start_rev)
            .version(format!("v{}", state.new_version));

        log::info!("clog: writing updated changelog");
        clog.write_changelog()?;

        PluginResponse::from_ok(vec![changelog_path.to_owned()])
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

pub fn analyze_single(commit_str: &str) -> Result<CommitType, failure::Error> {
    use CommitType::*;

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
