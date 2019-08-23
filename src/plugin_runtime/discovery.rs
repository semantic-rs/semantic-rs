use crate::plugin_support::{Plugin, PluginStep};

pub fn discover(plugin: &Plugin) -> Result<Vec<PluginStep>, failure::Error> {
    let response = plugin.as_interface().methods()?;
    Ok(response)
}
