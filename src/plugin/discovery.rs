use crate::config::CfgMap;
use crate::plugin::proto::request::PluginRequest;
use crate::plugin::{PluginStep, StartedPlugin};

pub struct CapabilitiesDiscovery;

impl CapabilitiesDiscovery {
    pub fn new() -> Self {
        CapabilitiesDiscovery
    }

    pub fn discover(
        &self,
        cfg_map: &CfgMap,
        plugin: &StartedPlugin,
    ) -> Result<Vec<PluginStep>, failure::Error> {
        let response = plugin
            .as_interface()
            .methods(PluginRequest::with_default_data(Clone::clone(cfg_map)))?;

        Ok(response)
    }
}
