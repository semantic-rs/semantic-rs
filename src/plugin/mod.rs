pub mod discovery;
pub mod dispatcher;
pub mod proto;
pub mod resolver;
pub mod starter;
pub mod traits;

pub use self::dispatcher::PluginDispatcher;
pub use self::traits::PluginInterface;

use serde::{Deserialize, Serialize};
use std::cell::{Ref, RefCell, RefMut};
use std::rc::Rc;

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
    Started(Plugin),
}

#[derive(Clone)]
pub struct Plugin {
    pub name: String,
    call: Rc<RefCell<Box<dyn PluginInterface>>>,
}

impl Plugin {
    pub fn new(plugin: Box<dyn PluginInterface>) -> Result<Self, failure::Error> {
        let name = plugin.name()?;
        let plugin = Plugin {
            name,
            call: Rc::new(RefCell::new(plugin)),
        };
        Ok(plugin)
    }

    pub fn as_interface(&self) -> Ref<Box<dyn PluginInterface>> {
        RefCell::borrow(&self.call)
    }

    pub fn as_interface_mut(&mut self) -> RefMut<Box<dyn PluginInterface>> {
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

    pub fn is_unresolved(&self) -> bool {
        match self {
            RawPluginState::Unresolved(_) => true,
            _ => false,
        }
    }

    pub fn is_started(&self) -> bool {
        match self {
            RawPluginState::Started(_) => true,
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
    Serialize, Deserialize, Debug, Copy, Clone, Eq, PartialEq, Hash, EnumString, IntoStaticStr,
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
            PluginStep::GetLastRelease | PluginStep::GenerateNotes | PluginStep::Commit => {
                PluginStepKind::Singleton
            }
        }
    }
}

#[derive(Copy, Clone, Debug)]
pub enum PluginStepKind {
    Singleton,
    Shared,
}
