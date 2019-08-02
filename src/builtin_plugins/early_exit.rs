use failure::Fail;
use serde::{Deserialize, Serialize};
use std::ops::Try;

use crate::plugin_support::flow::Value;
use crate::plugin_support::keys::{CURRENT_VERSION, NEXT_VERSION};
use crate::plugin_support::proto::{
    response::{self, PluginResponse},
    Version,
};
use crate::plugin_support::{PluginInterface, PluginStep};

pub struct EarlyExitPlugin {
    config: EarlyExitPluginConfig,
}

impl EarlyExitPlugin {
    pub fn new() -> Self {
        EarlyExitPlugin {
            config: EarlyExitPluginConfig::default(),
        }
    }
}

#[derive(Serialize, Deserialize)]
struct EarlyExitPluginConfig {
    current_version: Value<Version>,
    next_version: Value<semver::Version>,
}

impl Default for EarlyExitPluginConfig {
    fn default() -> Self {
        EarlyExitPluginConfig {
            current_version: Value::builder(CURRENT_VERSION)
                .required_at(PluginStep::DeriveNextVersion)
                .build(),
            next_version: Value::builder(NEXT_VERSION)
                .required_at(PluginStep::DeriveNextVersion)
                .protected()
                .build(),
        }
    }
}

impl PluginInterface for EarlyExitPlugin {
    fn name(&self) -> response::Name {
        PluginResponse::from_ok("early_exit".into())
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
        let methods = vec![PluginStep::DeriveNextVersion];
        PluginResponse::from_ok(methods)
    }

    fn derive_next_version(&mut self) -> response::Null {
        if self
            .config
            .current_version
            .as_value()
            .semver
            .as_ref()
            .map(|current| current == self.config.next_version.as_value())
            .unwrap_or(false)
        {
            log::info!("No version bump is required, you're all set!");
            return PluginResponse::from_error(
                EarlyExitError::EarlyExit("current and next versions are the same, nothing to do".into()).into(),
            );
        }

        PluginResponse::from_ok(())
    }
}

#[derive(Debug, Fail)]
pub enum EarlyExitError {
    #[fail(display = "Early exit, reason: {}", _0)]
    EarlyExit(String),
}
