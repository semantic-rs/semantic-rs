use super::Config;
use std::process::Command;

pub fn update_lockfile(config: &Config) -> bool {
    Command::new("cargo")
        .arg("fetch")
        .arg("--manifest-path")
        .arg(config.manifest_path())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

pub fn package(config: &Config) -> bool {
    Command::new("cargo")
        .arg("package")
        .arg("--manifest-path")
        .arg(config.manifest_path())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}
