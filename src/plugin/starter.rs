use crate::plugin::{Plugin, PluginName, PluginState, ResolvedPlugin, StartedPlugin};

pub struct PluginStarter {}

impl PluginStarter {
    pub fn new() -> Self {
        PluginStarter {}
    }
}

impl PluginStarter {
    pub fn start(&self, plugin: Plugin) -> Result<StartedPlugin, failure::Error> {
        let (name, state) = plugin.decompose();
        let started = match state {
            PluginState::Unresolved(_) => {
                panic!("all plugins must be resolved before calling Starter::start")
            }
            PluginState::Started(started) => started,
            PluginState::Resolved(resolved) => match resolved {
                ResolvedPlugin::Builtin(builtin) => StartedPlugin::new(builtin)?,
            },
        };
        Ok(started)
    }
}

trait Starter {
    fn start(
        &self,
        name: &PluginName,
        meta: &ResolvedPlugin,
    ) -> Result<StartedPlugin, failure::Error>;
}
