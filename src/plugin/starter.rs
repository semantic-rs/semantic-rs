use crate::plugin::{Plugin, RawPlugin, RawPluginState, ResolvedPlugin};

pub struct PluginStarter {}

impl PluginStarter {
    pub fn new() -> Self {
        PluginStarter {}
    }
}

impl PluginStarter {
    pub fn start(&self, plugin: RawPlugin) -> Result<Plugin, failure::Error> {
        let (name, state) = plugin.decompose();
        let started = match state {
            RawPluginState::Unresolved(_) => {
                panic!("all plugins must be resolved before calling Starter::start")
            }
            RawPluginState::Started(started) => started,
            RawPluginState::Resolved(resolved) => match resolved {
                ResolvedPlugin::Builtin(builtin) => Plugin::new(builtin)?,
            },
        };
        Ok(started)
    }
}

trait Starter {
    fn start(&self, name: &str, meta: &ResolvedPlugin) -> Result<Plugin, failure::Error>;
}
