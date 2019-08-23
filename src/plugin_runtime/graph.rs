use crate::config::{Config, Map, StepDefinition};
use crate::plugin_runtime::discovery::discover;
use crate::plugin_runtime::kernel::{InjectionTarget, PluginId};
use crate::plugin_support::flow::kv::{Key, ValueDefinition, ValueDefinitionMap, ValueState};
use crate::plugin_support::flow::{Availability, ProvisionCapability, Value};
use crate::plugin_support::{Plugin, PluginStep};
use failure::Fail;
use std::collections::VecDeque;
use strum::IntoEnumIterator;

pub type SourceKey = Key;
pub type DestKey = Key;

#[derive(Clone, Debug, PartialEq)]
pub struct Action {
    id: PluginId,
    kind: ActionKind,
}

impl Action {
    pub fn new(id: PluginId, kind: ActionKind) -> Self {
        Action { id, kind }
    }

    pub fn call(id: PluginId, step: PluginStep) -> Self {
        Action::new(id, ActionKind::Call(step))
    }

    pub fn get(id: PluginId, src_key: impl Into<String>) -> Self {
        Action::new(id, ActionKind::Get(src_key.into()))
    }

    pub fn set(id: PluginId, dst_key: impl Into<String>, src_key: impl Into<String>) -> Self {
        Action::new(id, ActionKind::Set(dst_key.into(), src_key.into()))
    }

    pub fn set_value(id: PluginId, dst_key: impl Into<String>, value: impl Into<serde_json::Value>) -> Self {
        Action::new(id, ActionKind::SetValue(dst_key.into(), value.into()))
    }

    pub fn require_config_entry(id: PluginId, dst_key: impl Into<String>) -> Self {
        Action::new(id, ActionKind::RequireConfigEntry(dst_key.into()))
    }

    pub fn require_env_value(id: PluginId, dst_key: impl Into<String>, src_key: impl Into<String>) -> Self {
        Action::new(id, ActionKind::RequireEnvValue(dst_key.into(), src_key.into()))
    }

    pub fn id(&self) -> PluginId {
        self.id
    }

    pub fn into_kind(self) -> ActionKind {
        self.kind
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum ActionKind {
    Call(PluginStep),
    Get(SourceKey),
    Set(DestKey, SourceKey),
    SetValue(DestKey, serde_json::Value),
    RequireConfigEntry(DestKey),
    RequireEnvValue(DestKey, SourceKey),
}

#[derive(Debug)]
pub struct PluginSequence {
    seq: Vec<Action>,
}

impl PluginSequence {
    pub fn new(
        plugins: &[Plugin],
        releaserc: &Config,
        injections: Vec<(PluginId, InjectionTarget)>,
        is_dry_run: bool,
    ) -> Result<Self, failure::Error> {
        // First -- collect data from plugins
        let names = collect_plugins_names(plugins);
        let configs = collect_plugins_initial_configuration(plugins)?;
        let caps = collect_plugins_provision_capabilities(plugins)?;
        let step_map = build_steps_to_plugins_map(
            releaserc,
            plugins,
            injections,
            collect_plugins_methods_capabilities(plugins)?,
        )?;

        // Then delegate that data to a builder
        let builder = PluginSequenceBuilder {
            names,
            configs,
            caps,
            releaserc: &releaserc.cfg,
            step_map,
        };

        builder.build(is_dry_run)
    }

    #[allow(dead_code)]
    pub fn iter(&self) -> impl Iterator<Item = &Action> {
        self.seq.iter()
    }

    pub fn into_iter(self) -> impl Iterator<Item = Action> {
        self.seq.into_iter()
    }
}

struct PluginSequenceBuilder<'a> {
    names: Vec<String>,
    configs: Vec<Map<String, Value<serde_json::Value>>>,
    caps: Vec<Vec<ProvisionCapability>>,
    releaserc: &'a ValueDefinitionMap,
    step_map: Map<PluginStep, Vec<PluginId>>,
}

impl<'a> PluginSequenceBuilder<'a> {
    fn build(mut self, is_dry_run: bool) -> Result<PluginSequence, failure::Error> {
        // Override default configs with values provided in releaserc.toml
        self.apply_releaserc_overrides();

        let mut seq = Vec::new();

        // Generate action sequence for dry steps
        for step in PluginStep::dry_steps() {
            let builder = StepSequenceBuilder::new(step, &self.names, &self.configs, &self.caps, &self.step_map);
            let step_seq = builder.build();
            seq.extend(step_seq.into_iter());
        }

        if !is_dry_run {
            for step in PluginStep::wet_steps() {
                let builder = StepSequenceBuilder::new(step, &self.names, &self.configs, &self.caps, &self.step_map);
                let step_seq = builder.build();
                seq.extend(step_seq.into_iter());
            }
        }

        Ok(PluginSequence { seq })
    }

    fn apply_releaserc_overrides(&mut self) {
        for (name, value) in self.releaserc.iter() {
            // Skip cfg entries that are not plugin configurations
            let id = match self.names.iter().position(|n| n == name) {
                Some(id) => id,
                None => continue,
            };

            let subtable: ValueDefinitionMap = match value {
                ValueDefinition::Value(value) => match serde_json::from_value(value.clone()) {
                    Ok(st) => st,
                    Err(err) => {
                        log::warn!("Failed to deserialize a table of key-value definitions: {}", err);
                        log::warn!("Configuration entry cfg.{} will be ignored", name);
                        continue;
                    }
                },
                ValueDefinition::From { .. } => {
                    log::warn!("'from' statements are not supported for top-level plugin configuration tables");
                    log::warn!("Configuration entry cfg.{} will be ignored", name);
                    continue;
                }
            };

            let cfg = &mut self.configs[id];
            for (dest_key, value_def) in subtable.iter() {
                if !cfg.contains_key(dest_key) {
                    log::warn!(
                        "Key cfg.{}.{} was defined in releaserc.toml but is not supported by plugin {:?}",
                        name,
                        dest_key,
                        name
                    );
                    continue;
                }

                match value_def {
                    ValueDefinition::Value(value) => {
                        let new = Value::builder(&dest_key).value(value.clone()).build();
                        cfg.insert(dest_key.clone(), new);
                    }
                    ValueDefinition::From {
                        required_at,
                        from_env,
                        key,
                    } => {
                        let mut new = Value::builder(&key);
                        if let Some(step) = required_at {
                            new.required_at(*step);
                        }
                        if *from_env {
                            new.load_from_env();
                        }
                        cfg.insert(key.clone(), new.build());
                    }
                }
            }
        }
    }
}

struct StepSequenceBuilder<'a> {
    step: PluginStep,
    names: &'a [String],
    caps: &'a [Vec<ProvisionCapability>],
    step_map: &'a Map<PluginStep, Vec<PluginId>>,

    seq: VecDeque<Action>,
    unresolved: Vec<Vec<(DestKey, SourceKey)>>,
    available_always: Map<SourceKey, Vec<PluginId>>,
    available_since: Map<SourceKey, Vec<(PluginId, PluginStep)>>,
    available_same_step: Map<SourceKey, Vec<PluginId>>,
    available_in_future: Map<SourceKey, Vec<(PluginId, PluginStep)>>,
}

impl<'a> StepSequenceBuilder<'a> {
    fn new(
        step: PluginStep,
        names: &'a [String],
        configs: &'a [Map<String, Value<serde_json::Value>>],
        caps: &'a [Vec<ProvisionCapability>],
        step_map: &'a Map<PluginStep, Vec<PluginId>>,
    ) -> Self {
        let mut seq = VecDeque::new();

        // Collect unresolved keys
        // Here are 2 keys for every plugin:
        // - destination: the key in the plugin config
        // - source: the key advertised by the plugin
        let unresolved = configs
            .iter()
            .enumerate()
            .map(|(dest_id, config)| {
                config
                    .iter()
                    .filter_map(|(dest_key, value)| match &value.state {
                        ValueState::Ready(value) => {
                            seq.push_back(Action::set_value(dest_id, dest_key, value.clone()));
                            None
                        }
                        ValueState::NeedsProvision(pr) => {
                            if pr.from_env {
                                seq.push_back(Action::require_env_value(dest_id, dest_key, &pr.key));
                                None
                            } else {
                                if pr.required_at > Some(step) {
                                    None
                                } else {
                                    Some((dest_key.clone(), pr.key.clone()))
                                }
                            }
                        }
                    })
                    .collect()
            })
            .collect();

        // TODO:
        // - error-handling for steps skipped in releaserc.toml (if plugin can provide data after step that's skipped -- that should be handled correctly)
        // - skip generating Call actions for steps that plugins do not implement
        // - rewrite tests

        // Collect a few maps from keys to plugins to make life easier
        let mut available_always = Map::new();
        let mut available_since = Map::new();
        let mut available_same_step = Map::new();
        let mut available_in_future = Map::new();
        caps.iter().enumerate().for_each(|(source_id, caps)| {
            caps.iter().for_each(|cap| match cap.when {
                Availability::Always => available_always
                    .entry(cap.key.clone())
                    .or_insert(Vec::new())
                    .push(source_id),
                Availability::AfterStep(after) => {
                    if after < step {
                        available_since
                            .entry(cap.key.clone())
                            .or_insert(Vec::new())
                            .push((source_id, after));
                    } else if after == step {
                        available_same_step
                            .entry(cap.key.clone())
                            .or_insert(Vec::new())
                            .push(source_id);
                    } else {
                        available_in_future
                            .entry(cap.key.clone())
                            .or_insert(Vec::new())
                            .push((source_id, after));
                    }
                }
            })
        });

        StepSequenceBuilder {
            step,
            names,
            caps,
            step_map,
            seq,
            unresolved,
            available_always,
            available_since,
            available_same_step,
            available_in_future,
        }
    }

    fn build(mut self) -> Vec<Action> {
        let mut seq = std::mem::replace(&mut self.seq, VecDeque::new());

        let unresolved = self.borrow_unresolved();

        // First -- resolve data that's trivially available from the previous step
        let unresolved = self.resolve_already_available(&mut seq, unresolved);

        // What's left unresolved is either
        // - inner-step dependencies, where one plugin in the current step depends on data provided by another after running the same step
        // - future-step dependencies, where data would only be available in future steps (then data should be in config)
        // - or data that should be available from the config, but is not there
        // Let's filter out the later 2 categories
        let unresolved = self.resolve_should_be_in_config(&mut seq, unresolved);

        // The next part is determining the sequence of running the plugins, and
        // since we do not do any reorders (as order is always determined by releaserc.toml)
        // this is not very hard
        //
        // If order is incorrect, that's an error and plugins should either be reordered
        // or the key should be defined in config manually
        self.resolve_same_step_and_build_call_sequence(&mut seq, unresolved);

        seq.into()
    }

    // Resolve data that's trivially available (Availability::Always or available since previous step)
    fn resolve_already_available<'b>(
        &self,
        seq: &mut VecDeque<Action>,
        unresolved: Vec<Vec<(&'b DestKey, &'b SourceKey)>>,
    ) -> Vec<Vec<(&'b DestKey, &'b SourceKey)>> {
        unresolved
            .into_iter()
            .enumerate()
            .map(|(dest_id, keys)| {
                keys.into_iter()
                    .filter_map(|(dest_key, source_key)| {
                        let mut resolved = false;

                        if let Some(plugins) = self.available_always.get(source_key) {
                            seq.extend(
                                plugins
                                    .iter()
                                    .filter(|&&source_id| source_id != dest_id)
                                    .map(|source_id| {
                                        Action::get(*source_id, source_key)
                                    }),
                            );
                            resolved = true;
                        }

                        if let Some(plugins) = self.available_since.get(source_key) {
                            for (src_id, step) in plugins {
                                if self.is_enabled_for_step(*src_id, *step) {
                                    seq.push_back(Action::get(*src_id, source_key));
                                    resolved = true;
                                } else {
                                    let dst_name = &self.names[dest_id];
                                    let src_name = &self.names[*src_id];
                                    log::warn!("Plugin {:?} requested key {:?}", dst_name, source_key);
                                    log::warn!("Matching source plugin {:?} can supply this key since step {:?}, but this step is not enabled for the source plugin", src_name, step);
                                }
                            }
                        }

                        if resolved {
                            seq.push_back(Action::set(
                                dest_id,
                                dest_key,
                                source_key,
                            ));
                            None
                        } else {
                            Some((dest_key, source_key))
                        }
                    })
                    .collect()
            })
            .collect()
    }

    // Resolve data that should be in config but isn't there
    fn resolve_should_be_in_config<'b>(
        &self,
        seq: &mut VecDeque<Action>,
        unresolved: Vec<Vec<(&'b DestKey, &'b SourceKey)>>,
    ) -> Vec<Vec<(&'b DestKey, &'b SourceKey)>> {
        unresolved.into_iter().enumerate().map(|(dest_id, keys)| {
            keys.into_iter().filter_map(|(dest_key, source_key)| {
                // Key must be resolved within the current step
                if self.available_same_step.contains_key(source_key) {
                    Some((dest_key, source_key))
                } else if let Some(plugins) = self.available_in_future.get(source_key) {
                    // Key is not available now, but would be in future steps.
                    let dest_plugin_name = &self.names[dest_id];
                    log::warn!("Plugin {:?} requested key {:?}", dest_plugin_name, source_key);
                    for (source_id, when) in plugins {
                        let source_plugin_name = &self.names[*source_id];
                        log::warn!("Matching source plugin {:?} can supply this key only after step {:?}, and the current step is {:?}", source_plugin_name, when, self.step);
                    }
                    log::warn!("The releaserc.toml entry cfg.{}.{} must be defined to proceed", dest_plugin_name, dest_key);
                    seq.push_front(Action::require_config_entry(dest_id, source_key));
                    None
                } else {
                    // Key cannot be supplied by plugins and must be defined in releaserc.toml
                    seq.push_front(Action::require_config_entry(dest_id, source_key));
                    None
                }
            }).collect()
        }).collect()
    }

    // Resolve data that should be in config but isn't there
    fn resolve_same_step_and_build_call_sequence<'b>(
        &self,
        seq: &mut VecDeque<Action>,
        unresolved: Vec<Vec<(&'b DestKey, &'b SourceKey)>>,
    ) {
        if self.step_map.get(&self.step).is_none() {
            return;
        }

        let plugins_to_run = self.step_map.get(&self.step).unwrap();

        // First option: every key is resolved. Then we just generate a number of Call actions.
        if unresolved.iter().all(Vec::is_empty) {
            seq.extend(plugins_to_run.iter().map(|&id| Action::call(id, self.step)));
            return;
        }

        // Second option: there are some inter-step resolutions being necessary,
        // so we check that the defined sequence of plugins is adequate for provisioning data
        let mut became_available = Map::new();
        for &dest_id in plugins_to_run {
            let unresolved_keys = &unresolved[dest_id];
            for cap in &self.caps[dest_id] {
                let available = match cap.when {
                    Availability::Always => true,
                    Availability::AfterStep(after) => after <= self.step && self.is_enabled(dest_id),
                };

                if available {
                    became_available
                        .entry(cap.key.as_str())
                        .or_insert(Vec::new())
                        .push(dest_id);
                }
            }

            for (dest_key, source_key) in unresolved_keys {
                if let Some(plugins) = became_available.get(source_key.as_str()) {
                    seq.extend(
                        plugins
                            .iter()
                            .filter(|&&source_id| source_id != dest_id)
                            .map(|source_id| Action::get(*source_id, *source_key)),
                    );
                    seq.push_back(Action::set(dest_id, *dest_key, *source_key));
                } else {
                    let dest_plugin_name = &self.names[dest_id];
                    log::error!("Plugin {:?} requested key {:?}", dest_plugin_name, source_key);
                    for source_id in self
                        .available_same_step
                        .get(source_key.as_str())
                        .expect("at this point only same-step keys should be unresolved. This is a bug.")
                    {
                        let source_plugin_name = &self.names[*source_id];
                        log::error!("Matching source plugin {:?} supplies this key at the current step ({:?}) but it's set to run after plugin {:?} in releaserc.toml", source_plugin_name, self.step, dest_plugin_name);
                    }
                    log::error!("Reorder the plugins in releaserc.toml or define the key manually.");
                    log::error!(
                        "The releaserc.toml entry cfg.{}.{} must be defined to proceed.",
                        dest_plugin_name,
                        dest_key
                    );
                    seq.push_front(Action::require_config_entry(dest_id, *dest_key));
                }
            }

            seq.push_back(Action::call(dest_id, self.step));
        }
    }

    fn is_enabled_for_step(&self, plugin_id: PluginId, step: PluginStep) -> bool {
        self.step_map
            .get(&step)
            .map(|s| s.contains(&plugin_id))
            .unwrap_or_default()
    }

    fn is_enabled(&self, plugin_id: PluginId) -> bool {
        self.is_enabled_for_step(plugin_id, self.step)
    }

    fn borrow_unresolved(&self) -> Vec<Vec<(&DestKey, &SourceKey)>> {
        self.unresolved
            .iter()
            .map(|list| list.iter().map(|(key, value)| (key, value)).collect())
            .collect()
    }
}

fn collect_plugins_names(plugins: &[Plugin]) -> Vec<String> {
    plugins.iter().map(|p| p.name.clone()).collect()
}

pub fn collect_plugins_initial_configuration(
    plugins: &[Plugin],
) -> Result<Vec<Map<String, Value<serde_json::Value>>>, failure::Error> {
    let mut configs = Vec::new();

    for plugin in plugins.iter() {
        let plugin_config = serde_json::from_value(plugin.as_interface().get_config()?)?;

        configs.push(plugin_config);
    }

    Ok(configs)
}

fn collect_plugins_provision_capabilities(plugins: &[Plugin]) -> Result<Vec<Vec<ProvisionCapability>>, failure::Error> {
    let mut caps = Vec::new();

    for plugin in plugins.iter() {
        let plugin_caps = plugin.as_interface().provision_capabilities()?;

        caps.push(plugin_caps);
    }

    Ok(caps)
}

fn collect_plugins_methods_capabilities(plugins: &[Plugin]) -> Result<Map<PluginStep, Vec<String>>, failure::Error> {
    let mut capabilities = Map::new();

    for plugin in plugins {
        let plugin_caps = discover(&plugin)?;
        for step in plugin_caps {
            capabilities
                .entry(step)
                .or_insert_with(Vec::new)
                .push(plugin.name.clone());
        }
    }

    Ok(capabilities)
}

fn build_steps_to_plugins_map(
    config: &Config,
    plugins: &[Plugin],
    injections: Vec<(PluginId, InjectionTarget)>,
    capabilities: Map<PluginStep, Vec<String>>,
) -> Result<Map<PluginStep, Vec<PluginId>>, failure::Error> {
    let mut map = Map::new();

    fn collect_ids_of_plugins_matching(plugins: &[Plugin], names: &[impl AsRef<str>]) -> Vec<usize> {
        names
            .iter()
            .filter_map(|name| plugins.iter().position(|plugin| plugin.name == name.as_ref()))
            .collect::<Vec<_>>()
    }

    for (step, step_def) in config.steps.iter() {
        match step_def {
            StepDefinition::Discover => {
                let names = capabilities.get(&step);

                let ids = if let Some(names) = names {
                    collect_ids_of_plugins_matching(&plugins[..], &names[..])
                } else {
                    Vec::new()
                };

                if ids.is_empty() {
                    log::warn!(
                        "Step '{}' is marked for auto-discovery, but no plugin implements this method",
                        step.as_str()
                    );
                }

                // Exclude injected plugins from discovery results
                let ids = ids
                    .into_iter()
                    .filter(|id| !injections.iter().any(|(x, _)| id == x))
                    .collect();

                map.insert(*step, ids);
            }
            StepDefinition::Singleton(plugin) => {
                let names = capabilities.get(&step).ok_or(Error::NoPluginsForStep(*step))?;

                if !names.contains(&plugin) {
                    return Err(Error::PluginDoesNotImplementStep(*step, plugin.to_string()).into());
                }

                let ids = collect_ids_of_plugins_matching(&plugins, &[plugin]);
                assert_eq!(ids.len(), 1);

                map.insert(*step, ids);
            }
            StepDefinition::Shared(list) => {
                if list.is_empty() {
                    continue;
                };

                let names = capabilities.get(&step).ok_or(Error::NoPluginsForStep(*step))?;

                for plugin in list {
                    if !names.contains(&plugin) {
                        return Err(Error::PluginDoesNotImplementStep(*step, plugin.to_string()).into());
                    }
                }

                let ids = collect_ids_of_plugins_matching(&plugins, &list[..]);
                assert_eq!(ids.len(), list.len());

                map.insert(*step, ids);
            }
        }
    }

    // Apply injections
    for (id, target) in injections {
        match target {
            InjectionTarget::BeforeStep(step) => map.entry(step).or_insert_with(Vec::new).insert(0, id),
            InjectionTarget::AfterStep(step) => map.entry(step).or_insert_with(Vec::new).push(id),
        }
    }

    Ok(map)
}

#[derive(Fail, Debug)]
#[rustfmt::skip]
enum Error {
    #[fail(display = "no plugins is capable of satisfying a non-null step {:?}", _0)]
    NoPluginsForStep(PluginStep),
    #[fail(display = "step {:?} requested plugin {:?}, but it does not implement this step", _0, 1)]
    PluginDoesNotImplementStep(PluginStep, String),
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::plugin_support::flow::{FlowError, ProvisionRequest};
    use crate::plugin_support::{
        proto::response::{self, PluginResponse},
        PluginInterface,
    };
    use std::ops::Try;

    fn dependent_provider_plugins() -> Vec<Plugin> {
        vec![
            Plugin::new(Box::new(self::test_plugins::Dependent::default())).unwrap(),
            Plugin::new(Box::new(self::test_plugins::Provider)).unwrap(),
        ]
    }

    #[test]
    fn collect_names() {
        let plugins = dependent_provider_plugins();
        let names = collect_plugins_names(&plugins);
        assert_eq!(2, names.len());
        for (id, plugin) in plugins.iter().enumerate() {
            assert_eq!(&plugin.name, &names[id]);
        }
    }

    #[test]
    fn collect_configs() {
        let plugins = dependent_provider_plugins();
        let configs = collect_plugins_initial_configuration(&plugins).unwrap();
        assert_eq!(2, configs.len());

        // Check dependent config
        let dependent_map = &configs[0];
        assert_eq!(dependent_map.len(), 1);
        assert!(dependent_map.contains_key("dest_key"));
        let dest_key_value = dependent_map.get("dest_key").unwrap();
        assert_eq!(
            dest_key_value.state,
            ValueState::NeedsProvision(ProvisionRequest {
                required_at: None,
                from_env: false,
                key: "source_key".to_string()
            })
        );

        // check provider config
        assert_eq!(configs[1].len(), 0);
    }

    #[test]
    fn collect_caps() {
        let plugins = dependent_provider_plugins();
        let caps = collect_plugins_provision_capabilities(&plugins).unwrap();
        assert_eq!(
            caps,
            vec![vec![], vec![ProvisionCapability::builder("source_key").build()]]
        );
    }

    #[test]
    fn steps_to_plugins_map() {
        env_logger::try_init().ok();

        let toml = r#"
            [plugins]
            dependent = "builtin"
            provider = "builtin"

            [steps]
            pre_flight = [ "dependent", "provider" ]
        "#;

        let config = toml::from_str(toml).unwrap();
        let plugins = dependent_provider_plugins();
        let caps = collect_plugins_methods_capabilities(&plugins).unwrap();

        let map = build_steps_to_plugins_map(&config, &plugins, vec![], caps).unwrap();

        let expected = vec![(PluginStep::PreFlight, vec![0, 1])].into_iter().collect();

        assert_eq!(map, expected);
    }

    #[test]
    fn steps_to_plugins_map_different_step_order() {
        env_logger::try_init().ok();

        let toml = r#"
            [plugins]
            dependent = "builtin"
            provider = "builtin"

            [steps]
            pre_flight = [ "provider", "dependent" ]
        "#;

        let config = toml::from_str(toml).unwrap();
        let plugins = dependent_provider_plugins();
        let caps = collect_plugins_methods_capabilities(&plugins).unwrap();

        let map = build_steps_to_plugins_map(&config, &plugins, vec![], caps).unwrap();

        let expected = vec![(PluginStep::PreFlight, vec![1, 0])].into_iter().collect();

        assert_eq!(map, expected);
    }

    #[test]
    fn steps_to_plugins_map_discovery() {
        env_logger::try_init().ok();

        let toml = r#"
            [plugins]
            dependent = "builtin"
            provider = "builtin"

            [steps]
            pre_flight = "discover"
        "#;

        let config = toml::from_str(toml).unwrap();
        let plugins = dependent_provider_plugins();
        let caps = collect_plugins_methods_capabilities(&plugins).unwrap();

        let map = build_steps_to_plugins_map(&config, &plugins, vec![], caps).unwrap();

        let expected = vec![(PluginStep::PreFlight, vec![0, 1])].into_iter().collect();

        assert_eq!(map, expected);
    }

    #[test]
    fn steps_to_plugins_map_with_injection() {
        env_logger::try_init().ok();

        let toml = r#"
            [plugins]
            dependent = "builtin"
            provider = "builtin"

            [steps]
            pre_flight = [ "provider", "dependent" ]
        "#;

        let config = toml::from_str(toml).unwrap();
        let mut plugins = dependent_provider_plugins();
        plugins.push(Plugin::new(Box::new(test_plugins::Injected)).unwrap());

        let caps = collect_plugins_methods_capabilities(&plugins).unwrap();
        let injections = vec![(2, InjectionTarget::BeforeStep(PluginStep::PreFlight))];

        let map = build_steps_to_plugins_map(&config, &plugins, injections, caps).unwrap();

        let expected = vec![(PluginStep::PreFlight, vec![2, 1, 0])].into_iter().collect();

        assert_eq!(map, expected);
    }

    #[test]
    fn steps_to_plugins_map_discovery_with_injection() {
        env_logger::try_init().ok();

        let toml = r#"
            [plugins]
            dependent = "builtin"
            provider = "builtin"

            [steps]
            pre_flight = "discover"
        "#;

        let config = toml::from_str(toml).unwrap();
        let mut plugins = dependent_provider_plugins();
        plugins.push(Plugin::new(Box::new(test_plugins::Injected)).unwrap());

        let caps = collect_plugins_methods_capabilities(&plugins).unwrap();
        let injections = vec![(2, InjectionTarget::BeforeStep(PluginStep::PreFlight))];

        let map = build_steps_to_plugins_map(&config, &plugins, injections, caps).unwrap();

        let expected = vec![(PluginStep::PreFlight, vec![2, 0, 1])].into_iter().collect();

        assert_eq!(map, expected);
    }

    #[test]
    fn steps_to_plugins_map_discovery_with_injection_to_other_step() {
        env_logger::try_init().ok();

        let toml = r#"
            [plugins]
            dependent = "builtin"
            provider = "builtin"

            [steps]
            pre_flight = "discover"
        "#;

        let config = toml::from_str(toml).unwrap();
        let mut plugins = dependent_provider_plugins();
        plugins.push(Plugin::new(Box::new(test_plugins::Injected)).unwrap());

        let caps = collect_plugins_methods_capabilities(&plugins).unwrap();
        let injections = vec![(2, InjectionTarget::BeforeStep(PluginStep::DeriveNextVersion))];

        let map = build_steps_to_plugins_map(&config, &plugins, injections, caps).unwrap();

        let expected = vec![
            (PluginStep::PreFlight, vec![0, 1]),
            (PluginStep::DeriveNextVersion, vec![2]),
        ]
        .into_iter()
        .collect();

        assert_eq!(map, expected);
    }

    #[test]
    #[ignore]
    // TODO: write sequence optimizer before testing the whole sequence
    fn build_sequence_for_dependent_provider() {
        env_logger::try_init().ok();

        let toml = r#"
            [plugins]
            dependent = "builtin"
            provider = "builtin"

            [steps]
            pre_flight = [ "dependent", "provider" ]
        "#;

        let config = toml::from_str(toml).unwrap();
        let PluginSequence { seq } =
            PluginSequence::new(&dependent_provider_plugins(), &config, vec![], false).unwrap();

        let correct_seq: Vec<Action> = PluginStep::iter()
            .flat_map(|step| {
                vec![
                    Action::get(1, "source_key"),
                    Action::set(0, "dest_key", "source_key"),
                    Action::call(0, step),
                    Action::call(1, step),
                ]
                .into_iter()
            })
            .collect();

        assert_eq!(seq, correct_seq);
    }

    #[test]
    #[ignore]
    // TODO: write sequence optimizer before testing the whole sequence
    fn build_sequence_for_dependent_provider_with_config_override() {
        let toml = r#"
            [plugins]
            dependent = "builtin"
            provider = "builtin"

            [steps]
            pre_flight = [ "dependent", "provider" ]

            [cfg]
            key = "value"

            [cfg.dependent]
            dest_key = "value"
        "#;

        let config = toml::from_str(toml).unwrap();
        let PluginSequence { seq } =
            PluginSequence::new(&dependent_provider_plugins(), &config, vec![], false).unwrap();

        let correct_seq: Vec<Action> = PluginStep::iter()
            .flat_map(|step| {
                vec![
                    Action::set_value(0, "dest_key", "value"),
                    Action::call(0, step),
                    Action::call(1, step),
                ]
                .into_iter()
            })
            .collect();

        assert_eq!(seq, correct_seq);
    }

    mod resolve {
        use super::*;

        mod already_available {
            use super::*;

            #[test]
            fn all_available() {
                let step = PluginStep::PreFlight;
                let names = vec!["one".into(), "two".into()];
                let configs = vec![
                    vec![("one_dst".into(), Value::builder("two_src").build())]
                        .into_iter()
                        .collect(),
                    vec![("two_dst".into(), Value::builder("one_src").build())]
                        .into_iter()
                        .collect(),
                ];
                let caps = vec![
                    vec![ProvisionCapability::builder("one_src").build()],
                    vec![ProvisionCapability::builder("two_src").build()],
                ];
                let step_map = vec![(step, vec![0, 1])].into_iter().collect();

                let ssb = StepSequenceBuilder::new(step, &names, &configs, &caps, &step_map);
                let unresolved = ssb.borrow_unresolved();
                let mut seq = VecDeque::new();

                let unresolved = ssb.resolve_already_available(&mut seq, unresolved);
                assert_eq!(unresolved, vec![vec![], vec![]]);
                assert_eq!(
                    Vec::from(seq),
                    vec![
                        Action::get(1, "two_src"),
                        Action::set(0, "one_dst", "two_src"),
                        Action::get(0, "one_src"),
                        Action::set(1, "two_dst", "one_src"),
                    ]
                );
            }

            #[test]
            fn same_key() {
                let step = PluginStep::PreFlight;
                let names = vec!["one".into(), "two".into()];
                let configs = vec![
                    vec![("dst".into(), Value::builder("src").build())]
                        .into_iter()
                        .collect(),
                    vec![("dst".into(), Value::builder("src").build())]
                        .into_iter()
                        .collect(),
                ];
                let caps = vec![
                    vec![ProvisionCapability::builder("src").build()],
                    vec![ProvisionCapability::builder("src").build()],
                ];
                let step_map = vec![(step, vec![0, 1])].into_iter().collect();

                let ssb = StepSequenceBuilder::new(step, &names, &configs, &caps, &step_map);
                let unresolved = ssb.borrow_unresolved();
                let mut seq = VecDeque::new();

                let unresolved = ssb.resolve_already_available(&mut seq, unresolved);
                assert_eq!(unresolved, vec![vec![], vec![]]);
                assert_eq!(
                    Vec::from(seq),
                    vec![
                        Action::get(1, "src"),
                        Action::set(0, "dst", "src"),
                        Action::get(0, "src"),
                        Action::set(1, "dst", "src"),
                    ]
                );
            }

            #[test]
            fn all_not_available() {
                let step = PluginStep::PreFlight;
                let names = vec!["one".into(), "two".into()];
                let configs = vec![
                    vec![("one_dst".into(), Value::builder("two_src").build())]
                        .into_iter()
                        .collect(),
                    vec![("two_dst".into(), Value::builder("one_src").build())]
                        .into_iter()
                        .collect(),
                ];
                let caps = vec![
                    vec![ProvisionCapability::builder("one_src")
                        .after_step(PluginStep::DeriveNextVersion)
                        .build()],
                    vec![ProvisionCapability::builder("two_src")
                        .after_step(PluginStep::Commit)
                        .build()],
                ];

                let step_map = vec![
                    (step, vec![0, 1]),
                    (PluginStep::DeriveNextVersion, vec![0]),
                    (PluginStep::Commit, vec![1]),
                ]
                .into_iter()
                .collect();

                let ssb = StepSequenceBuilder::new(step, &names, &configs, &caps, &step_map);
                let unresolved = ssb.borrow_unresolved();
                let mut seq = VecDeque::new();

                let unresolved = ssb.resolve_already_available(&mut seq, unresolved);
                assert_eq!(
                    unresolved,
                    vec![
                        vec![(&"one_dst".into(), &"two_src".into())],
                        vec![(&"two_dst".into(), &"one_src".into())],
                    ]
                );
                assert_eq!(Vec::from(seq), vec![]);
            }

            #[test]
            fn partially_not_available() {
                let step = PluginStep::PreFlight;
                let names = vec!["one".into(), "two".into()];
                let configs = vec![
                    vec![("one_dst".into(), Value::builder("two_src").build())]
                        .into_iter()
                        .collect(),
                    vec![("two_dst".into(), Value::builder("one_src").build())]
                        .into_iter()
                        .collect(),
                ];
                let caps = vec![
                    vec![ProvisionCapability::builder("one_src").build()],
                    vec![ProvisionCapability::builder("two_src")
                        .after_step(PluginStep::Commit)
                        .build()],
                ];
                let step_map = vec![(step, vec![0, 1]), (PluginStep::Commit, vec![1])]
                    .into_iter()
                    .collect();

                let ssb = StepSequenceBuilder::new(step, &names, &configs, &caps, &step_map);
                let unresolved = ssb.borrow_unresolved();
                let mut seq = VecDeque::new();

                let unresolved = ssb.resolve_already_available(&mut seq, unresolved);
                assert_eq!(unresolved, vec![vec![(&"one_dst".into(), &"two_src".into())], vec![],]);
                assert_eq!(
                    Vec::from(seq),
                    vec![Action::get(0, "one_src"), Action::set(1, "two_dst", "one_src"),]
                );
            }

            #[test]
            fn all_not_needed() {
                let step = PluginStep::PreFlight;
                let names = vec!["one".into(), "two".into()];
                let configs = vec![
                    vec![(
                        "one_dst".into(),
                        Value::builder("two_src").required_at(PluginStep::Commit).build(),
                    )]
                    .into_iter()
                    .collect(),
                    vec![(
                        "two_dst".into(),
                        Value::builder("one_src").required_at(PluginStep::GenerateNotes).build(),
                    )]
                    .into_iter()
                    .collect(),
                ];
                let caps = vec![
                    vec![ProvisionCapability::builder("one_src").build()],
                    vec![ProvisionCapability::builder("two_src").build()],
                ];
                let step_map = vec![
                    (step, vec![0, 1]),
                    (PluginStep::Commit, vec![0]),
                    (PluginStep::GenerateNotes, vec![1]),
                ]
                .into_iter()
                .collect();

                let ssb = StepSequenceBuilder::new(step, &names, &configs, &caps, &step_map);
                let unresolved = ssb.borrow_unresolved();
                let mut seq = VecDeque::new();

                let unresolved = ssb.resolve_already_available(&mut seq, unresolved);
                assert_eq!(unresolved, vec![vec![], vec![]]);
                assert_eq!(Vec::from(seq), vec![]);
            }

            #[test]
            fn partially_not_needed() {
                let step = PluginStep::PreFlight;
                let names = vec!["one".into(), "two".into()];
                let configs = vec![
                    vec![(
                        "one_dst".into(),
                        Value::builder("two_src").required_at(PluginStep::Commit).build(),
                    )]
                    .into_iter()
                    .collect(),
                    vec![("two_dst".into(), Value::builder("one_src").build())]
                        .into_iter()
                        .collect(),
                ];
                let caps = vec![
                    vec![ProvisionCapability::builder("one_src").build()],
                    vec![ProvisionCapability::builder("two_src").build()],
                ];
                let step_map = vec![(step, vec![0, 1]), (PluginStep::Commit, vec![0])]
                    .into_iter()
                    .collect();

                let ssb = StepSequenceBuilder::new(step, &names, &configs, &caps, &step_map);
                let unresolved = ssb.borrow_unresolved();
                let mut seq = VecDeque::new();

                let unresolved = ssb.resolve_already_available(&mut seq, unresolved);
                assert_eq!(unresolved, vec![vec![], vec![]]);
                assert_eq!(
                    Vec::from(seq),
                    vec![Action::get(0, "one_src"), Action::set(1, "two_dst", "one_src"),]
                );
            }
        }

        mod should_be_in_config {
            use super::*;

            #[test]
            fn same_step() {
                let step = PluginStep::PreFlight;
                let names = vec!["one".into(), "two".into()];
                let configs = vec![
                    vec![(
                        "one_dst".into(),
                        Value::builder("two_src").required_at(PluginStep::PreFlight).build(),
                    )]
                    .into_iter()
                    .collect(),
                    Map::new(),
                ];
                let caps = vec![
                    vec![],
                    vec![ProvisionCapability::builder("two_src")
                        .after_step(PluginStep::PreFlight)
                        .build()],
                ];

                let step_map = vec![(step, vec![0, 1])].into_iter().collect();

                let ssb = StepSequenceBuilder::new(step, &names, &configs, &caps, &step_map);
                let unresolved = ssb.borrow_unresolved();
                let mut seq = VecDeque::new();

                let unresolved = ssb.resolve_already_available(&mut seq, unresolved);
                assert_eq!(unresolved, vec![vec![(&"one_dst".into(), &"two_src".into())], vec![]]);
                assert_eq!(seq.len(), 0);

                let unresolved = ssb.resolve_should_be_in_config(&mut seq, unresolved);
                assert_eq!(unresolved, vec![vec![(&"one_dst".into(), &"two_src".into())], vec![]]);
                assert_eq!(seq.len(), 0);
            }

            #[test]
            fn unavailable() {
                let step = PluginStep::PreFlight;
                let names = vec!["one".into(), "two".into()];
                let configs = vec![
                    vec![("one_dst".into(), Value::builder("two_src").build())]
                        .into_iter()
                        .collect(),
                    Map::new(),
                ];
                let caps = vec![
                    vec![],
                    vec![ProvisionCapability::builder("two_src")
                        .after_step(PluginStep::Commit)
                        .build()],
                ];
                let step_map = vec![(step, vec![0, 1]), (PluginStep::Commit, vec![1])]
                    .into_iter()
                    .collect();

                let ssb = StepSequenceBuilder::new(step, &names, &configs, &caps, &step_map);
                let unresolved = ssb.borrow_unresolved();
                let mut seq = VecDeque::new();

                let unresolved = ssb.resolve_already_available(&mut seq, unresolved);
                assert_eq!(unresolved, vec![vec![(&"one_dst".into(), &"two_src".into())], vec![],]);
                assert_eq!(seq.len(), 0);

                let unresolved = ssb.resolve_should_be_in_config(&mut seq, unresolved);
                assert_eq!(unresolved, vec![vec![], vec![]]);
                assert_eq!(Vec::from(seq), vec![Action::require_config_entry(0, "two_src")]);
            }

            #[test]
            fn not_provided() {
                let step = PluginStep::PreFlight;
                let names = vec!["one".into(), "two".into()];
                let configs = vec![
                    vec![("one_dst".into(), Value::builder("two_src").build())]
                        .into_iter()
                        .collect(),
                    Map::new(),
                ];
                let caps = vec![vec![], vec![]];
                let step_map = vec![(step, vec![0, 1])].into_iter().collect();

                let ssb = StepSequenceBuilder::new(step, &names, &configs, &caps, &step_map);
                let unresolved = ssb.borrow_unresolved();
                let mut seq = VecDeque::new();

                let unresolved = ssb.resolve_already_available(&mut seq, unresolved);
                assert_eq!(unresolved, vec![vec![(&"one_dst".into(), &"two_src".into())], vec![],]);
                assert_eq!(seq.len(), 0);

                let unresolved = ssb.resolve_should_be_in_config(&mut seq, unresolved);
                assert_eq!(unresolved, vec![vec![], vec![]]);
                assert_eq!(Vec::from(seq), vec![Action::require_config_entry(0, "two_src")]);
            }
        }

        mod same_step_and_build_call_sequence {
            use super::*;

            #[test]
            fn correct_sequence() {
                let step = PluginStep::PreFlight;
                let names = vec!["one".into(), "two".into()];
                let configs = vec![
                    Map::new(),
                    vec![(
                        "two_dst".into(),
                        Value::builder("one_src").required_at(PluginStep::PreFlight).build(),
                    )]
                    .into_iter()
                    .collect(),
                ];
                let caps = vec![
                    vec![ProvisionCapability::builder("one_src")
                        .after_step(PluginStep::PreFlight)
                        .build()],
                    vec![],
                ];
                let step_map = vec![(step, vec![0, 1])].into_iter().collect();

                let ssb = StepSequenceBuilder::new(step, &names, &configs, &caps, &step_map);
                let unresolved = ssb.borrow_unresolved();
                let mut seq = VecDeque::new();

                let unresolved = ssb.resolve_already_available(&mut seq, unresolved);
                assert_eq!(unresolved, vec![vec![], vec![(&"two_dst".into(), &"one_src".into())],]);
                assert_eq!(seq.len(), 0);

                let unresolved = ssb.resolve_should_be_in_config(&mut seq, unresolved);
                assert_eq!(unresolved, vec![vec![], vec![(&"two_dst".into(), &"one_src".into())],]);
                assert_eq!(seq.len(), 0);

                ssb.resolve_same_step_and_build_call_sequence(&mut seq, unresolved);

                assert_eq!(
                    Vec::from(seq),
                    vec![
                        Action::call(0, PluginStep::PreFlight),
                        Action::get(0, "one_src"),
                        Action::set(1, "two_dst", "one_src"),
                        Action::call(1, PluginStep::PreFlight),
                    ]
                )
            }

            #[test]
            fn incorrect_sequence() {
                let step = PluginStep::PreFlight;
                let names = vec!["one".into(), "two".into()];
                let configs = vec![
                    vec![(
                        "one_dst".into(),
                        Value::builder("two_src").required_at(PluginStep::PreFlight).build(),
                    )]
                    .into_iter()
                    .collect(),
                    Map::new(),
                ];
                let caps = vec![
                    vec![],
                    vec![ProvisionCapability::builder("two_src")
                        .after_step(PluginStep::PreFlight)
                        .build()],
                ];
                let step_map = vec![(step, vec![0, 1])].into_iter().collect();

                let ssb = StepSequenceBuilder::new(step, &names, &configs, &caps, &step_map);
                let unresolved = ssb.borrow_unresolved();
                let mut seq = VecDeque::new();

                let unresolved = ssb.resolve_already_available(&mut seq, unresolved);
                assert_eq!(unresolved, vec![vec![(&"one_dst".into(), &"two_src".into())], vec![]]);
                assert_eq!(seq.len(), 0);

                let unresolved = ssb.resolve_should_be_in_config(&mut seq, unresolved);
                assert_eq!(unresolved, vec![vec![(&"one_dst".into(), &"two_src".into())], vec![]]);
                assert_eq!(seq.len(), 0);

                ssb.resolve_same_step_and_build_call_sequence(&mut seq, unresolved);

                assert_eq!(
                    Vec::from(seq),
                    vec![
                        Action::require_config_entry(0, "one_dst"),
                        Action::call(0, PluginStep::PreFlight),
                        Action::call(1, PluginStep::PreFlight),
                    ]
                )
            }
        }
    }

    mod test_plugins {
        use super::*;
        use serde::{Deserialize, Serialize};

        pub struct Dependent {
            config: DependentConfig,
        }

        #[derive(Serialize, Deserialize, Debug)]
        struct DependentConfig {
            dest_key: Value<String>,
        }

        impl Default for Dependent {
            fn default() -> Self {
                Dependent {
                    config: DependentConfig {
                        dest_key: Value::builder("source_key").build(),
                    },
                }
            }
        }

        impl PluginInterface for Dependent {
            fn name(&self) -> response::Name {
                PluginResponse::from_ok("dependent".into())
            }

            fn methods(&self) -> response::Methods {
                let methods = PluginStep::iter().collect();
                PluginResponse::from_ok(methods)
            }

            fn get_config(&self) -> response::Config {
                PluginResponse::from_ok(serde_json::to_value(&self.config).unwrap())
            }

            fn set_config(&mut self, config: serde_json::Value) -> response::Null {
                self.config = serde_json::from_value(config)?;
                PluginResponse::from_ok(())
            }
        }

        pub struct Provider;

        impl PluginInterface for Provider {
            fn name(&self) -> response::Name {
                PluginResponse::from_ok("provider".into())
            }

            fn methods(&self) -> response::Methods {
                let methods = PluginStep::iter().collect();
                PluginResponse::from_ok(methods)
            }

            fn provision_capabilities(&self) -> response::ProvisionCapabilities {
                PluginResponse::from_ok(vec![ProvisionCapability::builder("source_key").build()])
            }

            fn get_value(&self, key: &str) -> response::GetValue {
                match key {
                    "source_key" => PluginResponse::from_ok(serde_json::to_value("value").unwrap()),
                    other => PluginResponse::from_error(FlowError::KeyNotSupported(other.to_owned()).into()),
                }
            }

            fn get_config(&self) -> response::Config {
                PluginResponse::from_ok(serde_json::Value::Object(serde_json::Map::default()))
            }

            fn set_config(&mut self, _config: serde_json::Value) -> response::Null {
                unimplemented!()
            }
        }

        pub struct Injected;

        impl PluginInterface for Injected {
            fn name(&self) -> response::Name {
                PluginResponse::from_ok("injected".into())
            }

            fn methods(&self) -> response::Methods {
                let methods = PluginStep::iter().collect();
                PluginResponse::from_ok(methods)
            }

            fn provision_capabilities(&self) -> response::ProvisionCapabilities {
                PluginResponse::from_ok(vec![])
            }

            fn get_config(&self) -> response::Config {
                PluginResponse::from_ok(serde_json::Value::Object(serde_json::Map::default()))
            }

            fn set_config(&mut self, _config: serde_json::Value) -> response::Null {
                unimplemented!()
            }
        }
    }

}
