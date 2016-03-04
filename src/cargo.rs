use std::process::Command;

pub fn update_lockfile(repository_path: &str) -> bool {
    let manifest_path = format!("{}/Cargo.toml", repository_path);
    Command::new("cargo")
        .arg("fetch")
        .arg("--manifest-path")
        .arg(manifest_path)
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

pub fn package(repository_path: &str) -> bool {
    let manifest_path = format!("{}/Cargo.toml", repository_path);
    Command::new("cargo")
        .arg("package")
        .arg("--manifest-path")
        .arg(manifest_path)
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}
