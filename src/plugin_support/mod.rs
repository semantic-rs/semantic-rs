pub mod flow;
pub mod keys;
pub mod proto;
pub mod traits;

pub use self::traits::PluginInterface;

use serde::{Deserialize, Serialize};
use std::cell::{RefCell, RefMut};

pub struct RawPlugin {
    name: String,
    state: RawPluginState,
}

impl RawPlugin {
    pub fn new(name: String, state: RawPluginState) -> Self {
        RawPlugin { name, state }
    }

    pub fn name(&self) -> &String {
        &self.name
    }

    pub fn state(&self) -> &RawPluginState {
        &self.state
    }

    pub fn decompose(self) -> (String, RawPluginState) {
        (self.name, self.state)
    }
}

pub enum RawPluginState {
    Unresolved(UnresolvedPlugin),
    Resolved(ResolvedPlugin),
}

pub struct Plugin {
    pub name: String,
    call: RefCell<Box<dyn PluginInterface>>,
}

impl Plugin {
    pub fn new(plugin: Box<dyn PluginInterface>) -> Result<Self, failure::Error> {
        let name = plugin.name()?;
        let plugin = Plugin {
            name,
            call: RefCell::new(plugin),
        };
        Ok(plugin)
    }

    pub fn as_interface(&self) -> RefMut<Box<dyn PluginInterface>> {
        RefCell::borrow_mut(&self.call)
    }
}

impl RawPluginState {
    pub fn is_resolved(&self) -> bool {
        match self {
            RawPluginState::Resolved(_) => true,
            _ => false,
        }
    }

    pub fn as_unresolved(&self) -> Option<&UnresolvedPlugin> {
        match self {
            RawPluginState::Unresolved(unresolved) => Some(unresolved),
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

#[derive(
    Serialize,
    Deserialize,
    Debug,
    Copy,
    Clone,
    Ord,
    PartialOrd,
    Eq,
    PartialEq,
    Hash,
    EnumString,
    EnumIter,
    IntoStaticStr,
)]
#[serde(rename_all = "snake_case")]
#[strum(serialize_all = "snake_case")]
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
        self.into()
    }

    pub fn kind(self) -> PluginStepKind {
        match self {
            PluginStep::PreFlight
            | PluginStep::DeriveNextVersion
            | PluginStep::Prepare
            | PluginStep::VerifyRelease
            | PluginStep::Publish
            | PluginStep::Notify => PluginStepKind::Shared,
            PluginStep::GetLastRelease | PluginStep::GenerateNotes | PluginStep::Commit => PluginStepKind::Singleton,
        }
    }

    pub fn is_dry(self) -> bool {
        match self {
            PluginStep::PreFlight
            | PluginStep::GetLastRelease
            | PluginStep::DeriveNextVersion
            | PluginStep::GenerateNotes
            | PluginStep::Prepare
            | PluginStep::VerifyRelease => true,
            PluginStep::Publish | PluginStep::Notify | PluginStep::Commit => false,
        }
    }
}

#[derive(Copy, Clone, Debug)]
pub enum PluginStepKind {
    Singleton,
    Shared,
}
