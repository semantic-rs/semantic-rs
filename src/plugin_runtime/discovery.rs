use crate::plugin_support::{Plugin, PluginStep};

pub struct CapabilitiesDiscovery;

impl CapabilitiesDiscovery {
    pub fn new() -> Self {
        CapabilitiesDiscovery
    }

    pub fn discover(&self, plugin: &Plugin) -> Result<Vec<PluginStep>, failure::Error> {
        let response = plugin.as_interface().methods()?;

        Ok(response)
    }
}
