use std::fs::File;
use std::io::{Read, Write};
use std::ops::Try;
use std::path::{Path, PathBuf};
use std::process::Command;

use failure::Fail;
use serde::{Deserialize, Serialize};

use crate::plugin_support::flow::{FlowError, ProvisionCapability, Value};
use crate::plugin_support::keys::{DRY_RUN, FILES_TO_COMMIT, NEXT_VERSION, PROJECT_ROOT};
use crate::plugin_support::proto::response::{self, PluginResponse};
use crate::plugin_support::{PluginInterface, PluginStep};

pub struct RustPlugin {
    dry_run_guard: Option<DryRunGuard>,
    config: Config,
}

impl RustPlugin {
    pub fn new() -> Self {
        RustPlugin {
            dry_run_guard: None,
            config: Config::default(),
        }
    }
}

#[derive(Serialize, Deserialize)]
struct Config {
    project_root: Value<String>,
    dry_run: Value<bool>,
    token: Value<String>,
    next_version: Value<semver::Version>,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            project_root: Value::builder(PROJECT_ROOT).protected().build(),
            dry_run: Value::builder(DRY_RUN).protected().build(),
            token: Value::builder("CARGO_TOKEN").load_from_env().build(),
            next_version: Value::builder(NEXT_VERSION)
                .required_at(PluginStep::Prepare)
                .protected()
                .build(),
        }
    }
}

impl Drop for RustPlugin {
    fn drop(&mut self) {
        if let Some(guard) = self.dry_run_guard.as_ref() {
            log::info!("rust(dry-run): restoring original state of Cargo.toml");
            if let Err(err) = guard.cargo.write_manifest_raw(&guard.original_manifest) {
                log::error!("rust(dry-run): failed to restore original manifest, sorry x_x");
                log::error!("{}", err);
                log::info!(
                    "\nOriginal Cargo.toml: \n{}",
                    String::from_utf8_lossy(&guard.original_manifest)
                );
            }
        }
    }
}

struct DryRunGuard {
    original_manifest: Vec<u8>,
    cargo: Cargo,
}

impl PluginInterface for RustPlugin {
    fn name(&self) -> response::Name {
        PluginResponse::from_ok("rust".into())
    }

    fn provision_capabilities(&self) -> response::ProvisionCapabilities {
        PluginResponse::from_ok(vec![ProvisionCapability::builder(FILES_TO_COMMIT)
            .after_step(PluginStep::Prepare)
            .build()])
    }

    fn get_value(&self, key: &str) -> response::GetValue {
        let value = match key {
            "files_to_commit" => serde_json::to_value(vec!["Cargo.toml", "Cargo.lock"])?,
            _other => return PluginResponse::from_error(FlowError::KeyNotSupported(key.to_owned()).into()),
        };
        PluginResponse::from_ok(value)
    }

    fn get_config(&self) -> response::Config {
        PluginResponse::from_ok(serde_json::to_value(&self.config)?)
    }

    fn set_config(&mut self, config: serde_json::Value) -> response::Null {
        self.config = serde_json::from_value(config)?;
        PluginResponse::from_ok(())
    }

    fn methods(&self) -> response::Methods {
        let methods = vec![PluginStep::PreFlight, PluginStep::Prepare, PluginStep::VerifyRelease];
        PluginResponse::from_ok(methods)
    }

    fn pre_flight(&mut self) -> response::Null {
        let mut response = PluginResponse::builder();
        response.body(())
    }

    fn prepare(&mut self) -> response::Null {
        let project_root = self.config.project_root.as_value();
        let is_dry_run = *self.config.dry_run.as_value();

        let token = self.config.token.as_value();
        let cargo = Cargo::new(project_root, token)?;

        // If we're in the dry-run mode, we don't wanna change the Cargo.toml manifest,
        // so we save the original state of it, which would be written to
        if is_dry_run {
            log::info!("rust(dry-run): saving original state of Cargo.toml");

            let guard = DryRunGuard {
                original_manifest: cargo.load_manifest_raw()?,
                cargo: cargo.clone(),
            };

            self.dry_run_guard.replace(guard);
        }

        let next_version = self.config.next_version.as_value();
        cargo.set_version(next_version)?;

        PluginResponse::from_ok(())
    }

    fn verify_release(&mut self) -> response::Null {
        let project_root = self.config.project_root.as_value();

        let token = self.config.token.as_value();

        let cargo = Cargo::new(project_root, token)?;

        log::info!("Packaging new version, please wait...");
        cargo.package()?;
        log::info!("Package created successfully");

        PluginResponse::from_ok(())
    }

    fn publish(&mut self) -> response::Null {
        let project_root = self.config.project_root.as_value();

        let token = self.config.token.as_value();

        let cargo = Cargo::new(project_root, token)?;

        log::info!("Publishing new version, please wait...");
        cargo.publish()?;
        log::info!("Package published successfully");

        PluginResponse::from_ok(())
    }
}

#[derive(Clone, Debug)]
struct Cargo {
    manifest_path: PathBuf,
    token: String,
}

impl Cargo {
    pub fn new(project_root: &str, token: &str) -> Result<Self, failure::Error> {
        let manifest_path = Path::new(project_root).join("Cargo.toml");

        log::debug!("searching for manifest in {}", manifest_path.display());

        if !manifest_path.exists() || !manifest_path.is_file() {
            return Err(Error::CargoTomlNotFound(project_root.to_owned()).into());
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
            Err(Error::CargoCommandFailed(stdout, stderr).into())
        } else {
            Ok((stdout, stderr))
        }
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

    pub fn load_manifest_raw(&self) -> Result<Vec<u8>, failure::Error> {
        let mut manifest_file = File::open(&self.manifest_path)?;
        let mut contents = Vec::new();
        manifest_file.read_to_end(&mut contents)?;
        Ok(contents)
    }

    pub fn load_manifest(&self) -> Result<toml::Value, failure::Error> {
        Ok(toml::from_slice(&self.load_manifest_raw()?)?)
    }

    pub fn write_manifest_raw(&self, contents: &[u8]) -> Result<(), failure::Error> {
        let mut manifest_file = File::create(&self.manifest_path)?;
        manifest_file.write_all(contents)?;
        Ok(())
    }

    pub fn write_manifest(&self, manifest: toml::Value) -> Result<(), failure::Error> {
        let contents = toml::to_string_pretty(&manifest)?;
        self.write_manifest_raw(contents.as_bytes())
    }

    pub fn set_version(&self, version: &semver::Version) -> Result<(), failure::Error> {
        log::info!("Setting new version '{}' in Cargo.toml", version);

        let mut manifest = self.load_manifest()?;

        log::debug!("loaded Cargo.toml");

        {
            let root = manifest
                .as_table_mut()
                .ok_or(Error::InvalidManifest("expected table at root"))?;

            let package = root
                .get_mut("package")
                .ok_or(Error::InvalidManifest("package section not present"))?;
            let package = package
                .as_table_mut()
                .ok_or(Error::InvalidManifest("package section is expected to be map"))?;

            package.insert("version".into(), toml::Value::String(format!("{}", version)));
        }

        log::debug!("writing update to Cargo.toml");

        self.write_manifest(manifest)?;

        Ok(())
    }
}

#[derive(Fail, Debug)]
enum Error {
    #[fail(display = "Cargo.toml not found in {}", _0)]
    CargoTomlNotFound(String),
    #[fail(display = "failed to invoke cargo:\n\t\tSTDOUT:\n{}\n\t\tSTDERR:\n{}", _0, _1)]
    CargoCommandFailed(String, String),
    #[fail(display = "ill-formed Cargo.toml manifest: {}", _0)]
    InvalidManifest(&'static str),
}
