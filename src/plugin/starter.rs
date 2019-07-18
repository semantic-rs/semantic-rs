use crate::plugin::{Plugin, PluginInterface, PluginName, PluginState, ResolvedPlugin};

pub struct PluginStarter {}

impl PluginStarter {
    pub fn new() -> Self {
        PluginStarter {}
    }
}

impl PluginStarter {
    pub fn start(&self, plugin: Plugin) -> Result<Plugin, failure::Error> {
        let (name, state) = plugin.decompose();
        let started = match state {
            PluginState::Unresolved(_) => {
                panic!("all plugins must be resolved before calling Starter::start")
            }
            PluginState::Started(started) => started,
            PluginState::Resolved(resolved) => match resolved {
                ResolvedPlugin::Builtin(builtin) => builtin,
            },
        };
        Ok(Plugin::new(name, PluginState::Started(started)))
    }
}

trait Starter {
    fn start(
        &self,
        name: &PluginName,
        meta: &ResolvedPlugin,
    ) -> Result<Box<dyn PluginInterface>, failure::Error>;
}
