use std::collections::HashMap;
use std::ops::Try;
use std::process::Command;

use failure::Fail;

use crate::config::{CfgMap, CfgMapExt};
use crate::plugin::proto::{
    request,
    response::{self, PluginResponse},
    GitRevision, Version,
};
use crate::plugin::{PluginInterface, PluginStep};
use std::ffi::OsString;
use std::fs::File;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

pub struct RustPlugin {}

impl RustPlugin {
    pub fn new() -> Self {
        RustPlugin {}
    }
}

impl PluginInterface for RustPlugin {
    fn methods(&self, req: request::Methods) -> response::Methods {
        let mut methods = HashMap::new();
        methods.insert(PluginStep::PreFlight, true);
        methods.insert(PluginStep::Prepare, true);
        methods.insert(PluginStep::VerifyRelease, true);
        PluginResponse::from_ok(methods)
    }

    fn pre_flight(&self, params: request::PreFlight) -> response::PreFlight {
        let mut response = PluginResponse::builder();
        if !params.env.contains_key("CARGO_TOKEN") {
            response.error(RustPluginError::TokenUndefined);
        }
        response.body(()).build()
    }

    fn prepare(&self, params: request::Prepare) -> response::Prepare {
        let project_root = params.cfg_map.project_root()?;
        let token = params
            .env
            .get("CARGO_TOKEN")
            .ok_or(RustPluginError::TokenUndefined)?;

        let cargo = Cargo::new(project_root, token)?;

        cargo.set_version(params.data)?;

        PluginResponse::from_ok(())
    }

    fn verify_release(&self, params: request::VerifyRelease) -> response::VerifyRelease {
        let project_root = params.cfg_map.project_root()?;
        let token = params
            .env
            .get("CARGO_TOKEN")
            .ok_or(RustPluginError::TokenUndefined)?;

        let cargo = Cargo::new(project_root, token)?;

        log::info!("rust: packaging new version, please wait...");
        cargo.package()?;
        log::info!("rust: package created successfully");

        PluginResponse::from_ok(())
    }
}

struct Cargo {
    manifest_path: PathBuf,
    token: String,
}

impl Cargo {
    pub fn new(project_root: &str, token: &str) -> Result<Self, failure::Error> {
        let manifest_path = Path::new(project_root).join("Cargo.toml");

        log::debug!(
            "rust: searching for manifest in {}",
            manifest_path.display()
        );

        if !manifest_path.exists() || !manifest_path.is_file() {
            Err(RustPluginError::CargoTomlNotFound(project_root.to_owned()))?;
        }

        Ok(Cargo {
            manifest_path,
            token: token.to_owned(),
        })
    }

    fn run_command(command: &mut Command) -> Result<(String, String), failure::Error> {
        let output = command.output()?;
        let stdout = String::from_utf8(output.stdout)?;
        let stderr = String::from_utf8(output.stderr)?;

        if !output.status.success() {
            Err(RustPluginError::CargoCommandFailed(stdout, stderr).into())
        } else {
            Ok((stdout, stderr))
        }
    }

    pub fn update_lockfile(&self) -> Result<(), failure::Error> {
        let mut command = Command::new("cargo");
        let command = command
            .arg("fetch")
            .arg("--manifest-path")
            .arg(&self.manifest_path);

        Self::run_command(command)?;

        Ok(())
    }

    pub fn package(&self) -> Result<(), failure::Error> {
        let mut command = Command::new("cargo");
        let command = command
            .arg("package")
            .arg("--allow-dirty")
            .arg("--manifest-path")
            .arg(&self.manifest_path);

        Self::run_command(command)?;

        Ok(())
    }

    pub fn publish(&self) -> Result<(), failure::Error> {
        let mut command = Command::new("cargo");
        let command = command
            .arg("publish")
            .arg("--manifest-path")
            .arg(&self.manifest_path)
            .arg("--token")
            .arg(&self.token);

        Self::run_command(command)?;

        Ok(())
    }

    pub fn load_manifest(&self) -> Result<toml::Value, failure::Error> {
        let mut manifest_file = File::open(&self.manifest_path)?;
        let mut contents = Vec::new();
        manifest_file.read_to_end(&mut contents)?;
        Ok(toml::from_slice(&contents)?)
    }

    pub fn write_manifest(&self, manifest: toml::Value) -> Result<(), failure::Error> {
        let mut manifest_file = File::create(&self.manifest_path)?;
        let contents = toml::to_string_pretty(&manifest)?;
        manifest_file.write_all(contents.as_bytes())?;
        Ok(())
    }

    pub fn set_version(&self, version: semver::Version) -> Result<(), failure::Error> {
        log::info!("rust: setting new version '{}' in Cargo.toml", version);

        let mut manifest = self.load_manifest()?;

        log::debug!("rust: loaded Cargo.toml");

        {
            let root = manifest
                .as_table_mut()
                .ok_or(RustPluginError::InvalidManifest("expected table at root"))?;

            let package = root
                .get_mut("package")
                .ok_or(RustPluginError::InvalidManifest(
                    "package section not present",
                ))?;
            let package = package
                .as_table_mut()
                .ok_or(RustPluginError::InvalidManifest(
                    "package section is expected to be map",
                ))?;

            package.insert(
                "version".into(),
                toml::Value::String(format!("{}", version)),
            );
        }

        log::debug!("rust: writing update to Cargo.toml");

        self.write_manifest(manifest)?;

        Ok(())
    }
}

#[derive(Fail, Debug)]
pub enum RustPluginError {
    #[fail(display = "the CARGO_TOKEN environment variable is not configured")]
    TokenUndefined,
    #[fail(display = "Cargo.toml not found in {}", _0)]
    CargoTomlNotFound(String),
    #[fail(
        display = "failed to invoke cargo:\n\t\tSTDOUT:\n{}\n\t\tSTDERR:\n{}",
        _0, _1
    )]
    CargoCommandFailed(String, String),
    #[fail(display = "ill-formed Cargo.toml manifest: {}", _0)]
    InvalidManifest(&'static str),
}
