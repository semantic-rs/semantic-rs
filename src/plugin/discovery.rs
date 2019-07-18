use crate::config::CfgMap;
use crate::plugin::proto::request::PluginRequest;
use crate::plugin::{Plugin, PluginStep};

pub trait Discovery {
    fn discover(
        &self,
        cfg_map: &CfgMap,
        plugin: &Plugin,
    ) -> Result<Vec<PluginStep>, failure::Error>;
}

pub struct CapabilitiesDiscovery;

impl CapabilitiesDiscovery {
    pub fn new() -> Self {
        CapabilitiesDiscovery
    }
}

impl Discovery for CapabilitiesDiscovery {
    fn discover(
        &self,
        cfg_map: &CfgMap,
        plugin: &Plugin,
    ) -> Result<Vec<PluginStep>, failure::Error> {
        let response = plugin
            .as_interface()
            .methods(PluginRequest::with_default_data(Clone::clone(cfg_map)))?;

        let capabilities = response
            .into_iter()
            .filter(|(_, flag)| *flag)
            .map(|(step, _)| step)
            .collect();

        Ok(capabilities)
    }
}
