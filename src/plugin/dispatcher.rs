use std::fmt::Debug;

use super::{
    proto::{
        request::{self, PluginRequest},
        response::{self, PluginResponse},
        Version,
    },
    PluginName, PluginStep,
};

use crate::config::{CfgMap, Map};
use crate::plugin::{PluginInterface, StartedPlugin};

pub struct PluginDispatcher {
    config: CfgMap,
    map: Map<PluginStep, Vec<StartedPlugin>>,
}

impl PluginDispatcher {
    pub fn new(config: CfgMap, map: Map<PluginStep, Vec<StartedPlugin>>) -> Self {
        PluginDispatcher { config, map }
    }

    fn dispatch<RFR: Debug>(
        &self,
        step: PluginStep,
        call_fn: impl Fn(&dyn PluginInterface) -> PluginResponse<RFR>,
    ) -> DispatchedMultiResult<PluginResponse<RFR>> {
        let mut response_map = Map::new();

        if let Some(plugins) = self.mapped_plugins(step) {
            for plugin in plugins {
                log::info!("Invoking plugin '{}'", plugin.name);
                let response = call_fn(&**plugin.as_interface());
                log::debug!("{}: {:?}", plugin.name, response);
                response_map.insert(plugin.name.clone(), response);
            }
        }

        Ok(response_map)
    }

    fn dispatch_singleton<RFR: Debug>(
        &self,
        step: PluginStep,
        call_fn: impl FnOnce(&dyn PluginInterface) -> PluginResponse<RFR>,
    ) -> DispatchedSingletonResult<PluginResponse<RFR>> {
        let plugin = self.mapped_singleton(step);
        log::info!("Invoking singleton '{}'", plugin.name);
        let response = call_fn(&**plugin.as_interface());
        log::debug!("{}: {:?}", plugin.name, response);
        Ok((plugin.name.clone(), response))
    }

    fn mapped_plugins(&self, step: PluginStep) -> Option<impl Iterator<Item = StartedPlugin> + '_> {
        self.map.get(&step).map(|plugins| plugins.iter().cloned())
    }

    fn mapped_singleton(&self, step: PluginStep) -> StartedPlugin {
        let no_plugins_found_panic = || {
            panic!(
                "no plugins matching the singleton step {:?}: this is a bug, aborting.",
                step
            )
        };
        let too_many_plugins_panic = || {
            panic!(
                "more then one plugin matches the singleton step {:?}: this is a bug, aborting.",
                step
            )
        };

        let plugins = self.map.get(&step).unwrap_or_else(no_plugins_found_panic);

        if plugins.is_empty() {
            no_plugins_found_panic();
        }

        if plugins.len() != 1 {
            too_many_plugins_panic();
        }

        plugins[0].clone()
    }
}

pub type DispatchedMultiResult<T> = Result<Map<PluginName, T>, failure::Error>;
pub type DispatchedSingletonResult<T> = Result<(PluginName, T), failure::Error>;

impl PluginDispatcher {
    pub fn pre_flight(&self) -> DispatchedMultiResult<response::PreFlight> {
        self.dispatch(PluginStep::PreFlight, |p| {
            p.pre_flight(PluginRequest::with_default_data(self.config.clone()))
        })
    }

    pub fn get_last_release(&self) -> DispatchedSingletonResult<response::GetLastRelease> {
        self.dispatch_singleton(PluginStep::GetLastRelease, move |p| {
            p.get_last_release(PluginRequest::with_default_data(self.config.clone()))
        })
    }

    pub fn derive_next_version(
        &self,
        current_version: Version,
    ) -> DispatchedMultiResult<response::DeriveNextVersion> {
        self.dispatch(PluginStep::DeriveNextVersion, |p| {
            p.derive_next_version(PluginRequest::new(
                self.config.clone(),
                current_version.clone(),
            ))
        })
    }

    pub fn generate_notes(
        &self,
        params: request::GenerateNotesData,
    ) -> DispatchedMultiResult<response::GenerateNotes> {
        self.dispatch(PluginStep::GenerateNotes, |p| {
            p.generate_notes(PluginRequest::new(self.config.clone(), params.clone()))
        })
    }

    pub fn prepare(
        &self,
        params: request::PrepareData,
    ) -> DispatchedMultiResult<response::Prepare> {
        self.dispatch(PluginStep::Prepare, |p| {
            p.prepare(PluginRequest::new(self.config.clone(), params.clone()))
        })
    }

    pub fn verify_release(&self) -> DispatchedMultiResult<response::VerifyRelease> {
        self.dispatch(PluginStep::VerifyRelease, |p| {
            p.verify_release(PluginRequest::with_default_data(self.config.clone()))
        })
    }

    pub fn commit(
        &self,
        params: request::CommitData,
    ) -> DispatchedSingletonResult<response::Commit> {
        self.dispatch_singleton(PluginStep::Commit, move |p| {
            p.commit(PluginRequest::new(self.config.clone(), params))
        })
    }

    pub fn publish(
        &self,
        params: request::PublishData,
    ) -> DispatchedMultiResult<response::Publish> {
        self.dispatch(PluginStep::Publish, |p| {
            p.publish(PluginRequest::new(self.config.clone(), params.clone()))
        })
    }

    pub fn notify(&self, params: request::NotifyData) -> DispatchedMultiResult<response::Notify> {
        self.dispatch(PluginStep::Notify, |p| {
            p.notify(PluginRequest::new(self.config.clone(), params))
        })
    }
}
