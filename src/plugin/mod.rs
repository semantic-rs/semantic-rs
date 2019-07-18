pub mod discovery;
pub mod dispatcher;
pub mod proto;
pub mod resolver;
pub mod starter;
pub mod traits;

pub use self::dispatcher::PluginDispatcher;
pub use self::traits::PluginInterface;

use std::str::FromStr;

use serde::{Deserialize, Serialize};

pub type PluginName = String;

pub struct Plugin {
    name: PluginName,
    state: PluginState,
}

impl Plugin {
    pub fn new(name: PluginName, state: PluginState) -> Self {
        Plugin { name, state }
    }

    pub fn name(&self) -> &PluginName {
        &self.name
    }

    pub fn state(&self) -> &PluginState {
        &self.state
    }

    pub fn as_interface(&self) -> &dyn PluginInterface {
        match self.state() {
            PluginState::Started(executor) => &**executor,
            _other => panic!("plugin must be started before calling `Plugin::as_interface`"),
        }
    }

    pub fn decompose(self) -> (PluginName, PluginState) {
        (self.name, self.state)
    }
}

pub enum PluginState {
    Unresolved(UnresolvedPlugin),
    Resolved(ResolvedPlugin),
    Started(Box<dyn PluginInterface>),
}

impl PluginState {
    pub fn is_resolved(&self) -> bool {
        match self {
            PluginState::Resolved(_) => true,
            _ => false,
        }
    }

    pub fn is_unresolved(&self) -> bool {
        match self {
            PluginState::Unresolved(_) => true,
            _ => false,
        }
    }

    pub fn is_started(&self) -> bool {
        match self {
            PluginState::Started(_) => true,
            _ => false,
        }
    }

    pub fn as_unresolved(&self) -> Option<&UnresolvedPlugin> {
        match self {
            PluginState::Unresolved(unresolved) => Some(unresolved),
            _ => None,
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, Eq, PartialEq)]
#[serde(tag = "location")]
#[serde(rename_all = "lowercase")]
pub enum UnresolvedPlugin {
    Builtin,
    Cargo { package: String, version: String },
}

pub enum ResolvedPlugin {
    Builtin(Box<dyn PluginInterface>),
}

#[derive(Serialize, Deserialize, Debug, Copy, Clone, Eq, PartialEq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum PluginStep {
    PreFlight,
    GetLastRelease,
    DeriveNextVersion,
    GenerateNotes,
    Prepare,
    VerifyRelease,
    Commit,
    Publish,
    Notify,
}

impl PluginStep {
    pub fn as_str(self) -> &'static str {
        match self {
            PluginStep::PreFlight => "pre_flight",
            PluginStep::GetLastRelease => "get_last_release",
            PluginStep::DeriveNextVersion => "derive_next_release",
            PluginStep::GenerateNotes => "generate_notes",
            PluginStep::Prepare => "prepare",
            PluginStep::VerifyRelease => "verify_release",
            PluginStep::Commit => "commit",
            PluginStep::Publish => "publish",
            PluginStep::Notify => "notify",
        }
    }

    pub fn kind(self) -> PluginStepKind {
        match self {
            PluginStep::PreFlight
            | PluginStep::DeriveNextVersion
            | PluginStep::GenerateNotes
            | PluginStep::Prepare
            | PluginStep::VerifyRelease
            | PluginStep::Publish
            | PluginStep::Notify => PluginStepKind::Shared,
            PluginStep::GetLastRelease | PluginStep::Commit => PluginStepKind::Singleton,
        }
    }
}

impl FromStr for PluginStep {
    type Err = failure::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let variant = match s {
            "pre_flight" => PluginStep::PreFlight,
            "get_last_release" => PluginStep::GetLastRelease,
            "derive_next_version" => PluginStep::DeriveNextVersion,
            "generate_notes" => PluginStep::GenerateNotes,
            "prepare" => PluginStep::Prepare,
            "verify_release" => PluginStep::VerifyRelease,
            "commit" => PluginStep::Commit,
            "publish" => PluginStep::Publish,
            "notify" => PluginStep::Notify,
            other => Err(failure::format_err!("unknown step '{}'", other))?,
        };

        Ok(variant)
    }
}

#[derive(Copy, Clone, Debug)]
pub enum PluginStepKind {
    Singleton,
    Shared,
}
