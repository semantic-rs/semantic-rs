use std::io::BufWriter;
use std::ops::Try;
use std::path::{Path, PathBuf};

use clog::fmt::MarkdownWriter;
use clog::Clog;
use git2::{Commit, Repository};
use serde::{Deserialize, Serialize};

use crate::plugin_support::flow::{Availability, FlowError, ProvisionCapability, Value};
use crate::plugin_support::keys::{
    CURRENT_VERSION, DRY_RUN, FILES_TO_COMMIT, NEXT_VERSION, PROJECT_ROOT, RELEASE_NOTES,
};
use crate::plugin_support::proto::{
    response::{self, PluginResponse},
    Version,
};
use crate::plugin_support::{PluginInterface, PluginStep};

pub struct ClogPlugin {
    config: Config,
    state: State,
    dry_run_guard: Option<DryRunGuard>,
}

impl ClogPlugin {
    pub fn new() -> Self {
        ClogPlugin {
            config: Config::default(),
            state: State::default(),
            dry_run_guard: None,
        }
    }
}

#[derive(Default)]
struct State {
    release_notes: Option<String>,
    next_version: Option<semver::Version>,
}

impl Drop for ClogPlugin {
    fn drop(&mut self) {
        if let Some(guard) = self.dry_run_guard.as_ref() {
            log::info!("clog(dry-run): restoring original state of changelog file");

            let result = if let Some(original_changelog) = &guard.original_changelog {
                std::fs::write(&guard.changelog_path, original_changelog)
            } else {
                std::fs::remove_file(&guard.changelog_path)
            };

            if let Err(err) = result {
                log::error!("failed to restore original changelog, sorry x_x");
                log::error!("{}", err);
                if let Some(oc) = &guard.original_changelog {
                    log::info!("\nOriginal changelog: \n{}", String::from_utf8_lossy(oc));
                } else {
                    log::info!("There is no previous state changelog file (not found)");
                }
            }
        }
    }
}

struct DryRunGuard {
    changelog_path: PathBuf,
    original_changelog: Option<Vec<u8>>,
}

#[derive(Serialize, Deserialize)]
struct Config {
    changelog: Value<String>,
    ignore: Value<Vec<String>>,
    project_root: Value<String>,
    dry_run: Value<bool>,
    current_version: Value<Version>,
    next_version: Value<semver::Version>,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            changelog: Value::builder("changelog").value("Changelog.md".into()).build(),
            ignore: Value::builder("ignore").default_value().build(),
            project_root: Value::builder(PROJECT_ROOT).protected().build(),
            dry_run: Value::builder(DRY_RUN).protected().build(),
            current_version: Value::builder(CURRENT_VERSION)
                .required_at(PluginStep::DeriveNextVersion)
                .build(),
            next_version: Value::builder(NEXT_VERSION)
                .required_at(PluginStep::GenerateNotes)
                .protected()
                .build(),
        }
    }
}

impl PluginInterface for ClogPlugin {
    fn name(&self) -> response::Name {
        PluginResponse::from_ok("clog".into())
    }

    fn provision_capabilities(&self) -> response::ProvisionCapabilities {
        PluginResponse::from_ok(vec![
            ProvisionCapability::builder(RELEASE_NOTES)
                .after_step(PluginStep::GenerateNotes)
                .build(),
            ProvisionCapability::builder(NEXT_VERSION)
                .after_step(PluginStep::DeriveNextVersion)
                .build(),
            ProvisionCapability::builder(FILES_TO_COMMIT)
                .after_step(PluginStep::Prepare)
                .build(),
        ])
    }

    fn get_value(&self, key: &str) -> response::GetValue {
        match key {
            "release_notes" => {
                let notes = self.state.release_notes.as_ref().ok_or_else(|| {
                    FlowError::DataNotAvailableYet(key.to_owned(), Availability::AfterStep(PluginStep::GenerateNotes))
                })?;

                PluginResponse::from_ok(serde_json::to_value(notes)?)
            }
            "next_version" => {
                let next_version = self.state.next_version.as_ref().ok_or_else(|| {
                    FlowError::DataNotAvailableYet(
                        key.to_owned(),
                        Availability::AfterStep(PluginStep::DeriveNextVersion),
                    )
                })?;

                PluginResponse::from_ok(serde_json::to_value(next_version)?)
            }
            "files_to_commit" => {
                let changelog_path = self.config.changelog.as_value();
                PluginResponse::from_ok(serde_json::to_value(vec![changelog_path])?)
            }
            other => PluginResponse::from_error(FlowError::KeyNotSupported(other.to_owned()).into()),
        }
    }

    fn get_config(&self) -> response::Config {
        let json = serde_json::to_value(&self.config)?;
        PluginResponse::from_ok(json)
    }

    fn set_config(&mut self, config: serde_json::Value) -> response::Null {
        self.config = serde_json::from_value(config)?;
        PluginResponse::from_ok(())
    }

    fn methods(&self) -> response::Methods {
        let methods = vec![
            PluginStep::PreFlight,
            PluginStep::DeriveNextVersion,
            PluginStep::GenerateNotes,
            PluginStep::Prepare,
        ];
        PluginResponse::from_ok(methods)
    }

    fn pre_flight(&mut self) -> response::Null {
        PluginResponse::from_ok(())
    }

    fn derive_next_version(&mut self) -> response::Null {
        let cfg = &self.config;
        let project_root = cfg.project_root.as_value();
        let current_version = cfg.current_version.as_value();
        let ignore = cfg.ignore.as_value();

        let bump = match &current_version.semver {
            None => CommitType::Major,
            Some(_) => version_bump_since_rev(&project_root, &current_version.rev, &ignore)?,
        };

        let next_version = match current_version.semver.clone() {
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

        self.state.next_version.replace(next_version.clone());

        PluginResponse::from_ok(())
    }

    fn generate_notes(&mut self) -> response::Null {
        let changelog = {
            let project_root = self.config.project_root.as_value();
            let current_version = self.config.current_version.as_value();
            let next_version = self.config.next_version.as_value();

            let changelog = generate_changelog(project_root, &current_version.rev, next_version)?;

            log::info!("Changelog for {}..{}", current_version.rev, next_version);
            log::info!("---------------------------------------------------");
            log::info!("{}", changelog);
            log::info!("---------------------------------------------------");

            changelog
        };

        // Store this request as state
        self.state.release_notes.replace(changelog.clone());

        PluginResponse::from_ok(())
    }

    fn prepare(&mut self) -> response::Null {
        let cfg = &self.config;
        let changelog_path = cfg.changelog.as_value();
        let repo_path = cfg.project_root.as_value();
        let is_dry_run = *cfg.dry_run.as_value();
        let current_version = cfg.current_version.as_value();
        let next_version = cfg.next_version.as_value();

        // Safely store the original changelog for restoration after dry-run is finished
        if is_dry_run {
            log::info!("clog(dry-run): saving original state of changelog file");
            let original_changelog = std::fs::read(&changelog_path).ok();
            self.dry_run_guard.replace(DryRunGuard {
                changelog_path: Path::new(changelog_path).to_owned(),
                original_changelog,
            });
        }

        let mut clog = Clog::with_dir(repo_path)?;
        clog.changelog(changelog_path)
            .from(&current_version.rev)
            .version(format!("v{}", next_version));

        log::info!("Writing updated changelog");
        clog.write_changelog()?;

        PluginResponse::from_ok(())
    }
}

fn version_bump_since_rev(path: &str, rev: &str, ignore: &[String]) -> Result<CommitType, failure::Error> {
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
        .map(|c| analyze_single(&c, ignore).expect("commit analysis failed"))
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

pub fn analyze_single(commit_str: &str, ignore: &[String]) -> Result<CommitType, failure::Error> {
    use CommitType::*;

    let message = commit_str.trim().split_terminator('\n').nth(1);

    let clog = Clog::new().expect("Clog initialization failed");
    let commit = clog.parse_raw_commit(commit_str);

    if !commit.breaks.is_empty() {
        return Ok(Major);
    }

    if ignore.contains(&commit.component.to_ascii_lowercase()) {
        return Ok(Unknown);
    }

    let commit_type = match &commit.commit_type[..] {
        "Features" => Minor,
        "Bug Fixes" => Patch,
        _ => Unknown,
    };

    if let Some(message) = message {
        log::trace!("derived commit type {:?} for {}", commit_type, message);
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
        assert_eq!(CommitType::Unknown, analyze_single(commit, &[]).unwrap());
    }

    #[test]
    fn patch_commit() {
        let commit = "0\nfix: This commit fixes a bug";
        assert_eq!(CommitType::Patch, analyze_single(commit, &[]).unwrap());
    }

    #[test]
    fn minor_commit() {
        let commit = "0\nfeat: This commit introduces a new feature";
        assert_eq!(CommitType::Minor, analyze_single(commit, &[]).unwrap());
    }

    #[test]
    fn major_commit() {
        let commit = "0\nfeat: This commits breaks something\nBREAKING CHANGE: breaks things";
        assert_eq!(CommitType::Major, analyze_single(commit, &[]).unwrap());
    }

    #[test]
    fn ignored_component() {
        let commit = "0\nfeat(ci): This commits should be ignored";
        assert_eq!(CommitType::Unknown, analyze_single(commit, &["ci".into()]).unwrap());
    }
}
