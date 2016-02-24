use super::Config;
use std::process::Command;

pub fn update_lockfile(config: &Config) -> bool {
    let mut manifest_path = config.repository_path.clone();
    manifest_path.push("Cargo.toml");

    Command::new("cargo")
        .arg("fetch")
        .arg("--manifest-path")
        .arg(manifest_path)
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

pub fn package(config: &Config) -> bool {
    let mut manifest_path = config.repository_path.clone();
    manifest_path.push("Cargo.toml");

    Command::new("cargo")
        .arg("package")
        .arg("--manifest-path")
        .arg(manifest_path)
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}
