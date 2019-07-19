use std::mem;
use std::ops::Try;

use failure::Fail;

use crate::config::{CfgMap, CfgMapExt, Config, Map, PluginDefinitionMap, StepDefinition};
use crate::plugin::discovery::CapabilitiesDiscovery;
use crate::plugin::proto::Version;
use crate::plugin::proto::{request, response::PluginResponse};
use crate::plugin::resolver::PluginResolver;
use crate::plugin::starter::PluginStarter;
use crate::plugin::{Plugin, PluginDispatcher, PluginStep, RawPlugin, RawPluginState};

const STEPS_DRY: &[PluginStep] = &[
    PluginStep::PreFlight,
    PluginStep::GetLastRelease,
    PluginStep::DeriveNextVersion,
    PluginStep::GenerateNotes,
    PluginStep::Prepare,
    PluginStep::VerifyRelease,
];

const STEPS_WET: &[PluginStep] = &[PluginStep::Commit, PluginStep::Publish, PluginStep::Notify];

pub struct Kernel {
    dispatcher: PluginDispatcher,
    is_dry_run: bool,
}

impl Kernel {
    pub fn builder(config: Config) -> KernelBuilder {
        KernelBuilder {
            config,
            additional_plugins: vec![],
        }
    }

    pub fn run(self) -> Result<(), failure::Error> {
        let mut data = KernelData::default();

        let mut run_step = |step: PluginStep| -> Result<(), failure::Error> {
            log::info!("Running step '{}'", step.as_str());

            step.execute(&self, &mut data).map_err(|err| {
                log::error!("Step {:?} failed", step);
                err
            })?;

            Ok(())
        };

        // Run through the "dry" steps
        for &step in STEPS_DRY {
            run_step(step)?;
        }

        if self.is_dry_run {
            log::info!("DRY RUN: skipping steps {:?}", STEPS_WET);
        } else {
            for &step in STEPS_WET {
                run_step(step)?
            }
        }

        Ok(())
    }
}

pub struct KernelBuilder {
    config: Config,
    additional_plugins: Vec<RawPlugin>,
}

impl KernelBuilder {
    pub fn plugin(&mut self, plugin: RawPlugin) -> &mut Self {
        self.additional_plugins.push(plugin);
        self
    }

    pub fn build(&mut self) -> Result<Kernel, failure::Error> {
        // Move PluginDefinitions out of config and convert them to Plugins
        let plugins = mem::replace(&mut self.config.plugins, Map::new());
        let mut plugins = Self::plugin_def_map_to_vec(plugins);

        // Append plugins from config to additional plugins
        // Order matters here 'cause additional plugins
        // MUST run before external plugins from Config
        self.additional_plugins.extend(plugins.drain(..));
        let plugins = mem::replace(&mut self.additional_plugins, Vec::new());

        // Resolve stage
        let plugins = Self::resolve_plugins(plugins)?;
        Self::check_all_resolved(&plugins)?;
        log::info!("All plugins resolved");

        // Starting stage
        let plugins = Self::start_plugins(plugins)?;
        log::info!("All plugins started");

        // Discovering plugins capabilities
        let capabilities = Self::discover_capabilities(&self.config.cfg, &plugins)?;

        // Building a steps to plugins map
        let steps_to_plugins =
            Self::build_steps_to_plugins_map(&self.config, plugins, capabilities)?;

        // Extract some configuration values from CfgMap
        let cfg_map = mem::replace(&mut self.config.cfg, CfgMap::new());
        let is_dry_run = cfg_map.is_dry_run()?;

        // Create Dispatcher
        let dispatcher = PluginDispatcher::new(cfg_map, steps_to_plugins);

        Ok(Kernel {
            dispatcher,
            is_dry_run,
        })
    }

    fn plugin_def_map_to_vec(plugins: PluginDefinitionMap) -> Vec<RawPlugin> {
        plugins
            .into_iter()
            .map(|(name, def)| RawPlugin::new(name, RawPluginState::Unresolved(def.into_full())))
            .collect()
    }

    fn resolve_plugins(plugins: Vec<RawPlugin>) -> Result<Vec<RawPlugin>, failure::Error> {
        log::info!("Resolving plugins");
        let resolver = PluginResolver::new();
        let plugins = plugins
            .into_iter()
            .map(|p| resolver.resolve(p))
            .collect::<Result<_, _>>()?;
        Ok(plugins)
    }

    fn start_plugins(plugins: Vec<RawPlugin>) -> Result<Vec<Plugin>, failure::Error> {
        log::info!("Starting plugins");
        let starter = PluginStarter::new();
        let plugins = plugins
            .into_iter()
            .map(|p| starter.start(p))
            .collect::<Result<_, _>>()?;
        Ok(plugins)
    }

    fn discover_capabilities(
        cfg_map: &CfgMap,
        plugins: &[Plugin],
    ) -> Result<Map<PluginStep, Vec<String>>, failure::Error> {
        let discovery = CapabilitiesDiscovery::new();
        let mut capabilities = Map::new();

        for plugin in plugins {
            let plugin_caps = discovery.discover(cfg_map, &plugin)?;
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
        plugins: Vec<Plugin>,
        capabilities: Map<PluginStep, Vec<String>>,
    ) -> Result<Map<PluginStep, Vec<Plugin>>, failure::Error> {
        let mut map = Map::new();

        fn copy_plugins_matching(plugins: &[Plugin], names: &[impl AsRef<str>]) -> Vec<Plugin> {
            plugins
                .iter()
                .filter(|p| names.iter().map(AsRef::as_ref).any(|n| n == p.name))
                .cloned()
                .collect::<Vec<_>>()
        }

        // TODO: Store plugins in Arc links to make it possible to have copies in different steps
        for (step, step_def) in config.steps.iter() {
            match step_def {
                StepDefinition::Discover => {
                    let names = capabilities.get(&step);

                    let plugins = if let Some(names) = names {
                        copy_plugins_matching(&plugins[..], &names[..])
                    } else {
                        Vec::new()
                    };

                    if plugins.is_empty() {
                        log::warn!("Step '{}' is marked for auto-discovery, but no plugin implements this method", step.as_str());
                    }

                    map.insert(*step, plugins);
                }
                StepDefinition::Singleton(plugin) => {
                    let names = capabilities
                        .get(&step)
                        .ok_or(KernelError::NoPluginsForStep(*step))?;

                    if !names.contains(&plugin) {
                        Err(KernelError::PluginDoesNotImplementStep(
                            *step,
                            plugin.to_string(),
                        ))?
                    }

                    let plugins = copy_plugins_matching(&plugins, &[plugin]);
                    assert_eq!(plugins.len(), 1);

                    map.insert(*step, plugins);
                }
                StepDefinition::Shared(list) => {
                    if list.is_empty() {
                        continue;
                    };

                    let names = capabilities
                        .get(&step)
                        .ok_or(KernelError::NoPluginsForStep(*step))?;

                    for plugin in list {
                        if !names.contains(&plugin) {
                            Err(KernelError::PluginDoesNotImplementStep(
                                *step,
                                plugin.to_string(),
                            ))?
                        }
                    }

                    let plugins = copy_plugins_matching(&plugins, &list[..]);
                    assert_eq!(plugins.len(), list.len());

                    map.insert(*step, plugins);
                }
            }
        }

        Ok(map)
    }

    fn check_all_resolved(plugins: &[RawPlugin]) -> Result<(), failure::Error> {
        let unresolved = Self::list_not_resolved_plugins(plugins);
        if unresolved.is_empty() {
            Ok(())
        } else {
            Err(KernelError::FailedToResolvePlugins(unresolved).into())
        }
    }

    fn check_all_started(plugins: &[RawPlugin]) -> Result<(), failure::Error> {
        let not_started = Self::list_not_started_plugins(plugins);
        if not_started.is_empty() {
            Ok(())
        } else {
            Err(KernelError::FailedToStartPlugins(not_started).into())
        }
    }

    fn list_not_resolved_plugins(plugins: &[RawPlugin]) -> Vec<String> {
        Self::list_all_plugins_that(plugins, |plugin| match plugin.state() {
            RawPluginState::Unresolved(_) => true,
            RawPluginState::Resolved(_) | RawPluginState::Started(_) => false,
        })
    }

    fn list_not_started_plugins(plugins: &[RawPlugin]) -> Vec<String> {
        Self::list_all_plugins_that(plugins, |plugin| match plugin.state() {
            RawPluginState::Unresolved(_) | RawPluginState::Resolved(_) => true,
            RawPluginState::Started(_) => false,
        })
    }

    fn list_all_plugins_that(
        plugins: &[RawPlugin],
        filter: impl Fn(&RawPlugin) -> bool,
    ) -> Vec<String> {
        plugins
            .iter()
            .filter_map(|plugin| {
                if filter(plugin) {
                    Some(plugin.name().clone())
                } else {
                    None
                }
            })
            .collect()
    }
}

#[derive(Fail, Debug)]
enum KernelError {
    #[fail(display = "failed to resolve some modules: \n{:#?}", _0)]
    FailedToResolvePlugins(Vec<String>),
    #[fail(display = "failed to start some modules: \n{:#?}", _0)]
    FailedToStartPlugins(Vec<String>),
    #[fail(
        display = "no plugins is capable of satisfying a non-null step {:?}",
        _0
    )]
    NoPluginsForStep(PluginStep),
    #[fail(
        display = "step {:?} requested plugin {:?}, but it does not implement this step",
        _0, 1
    )]
    PluginDoesNotImplementStep(PluginStep, String),
    #[fail(
        display = "required data '{}' was not provided by the previous steps",
        _0
    )]
    MissingRequiredData(&'static str),
}

#[derive(Default)]
struct KernelData {
    last_version: Option<Version>,
    next_version: Option<semver::Version>,
    changelog: Option<String>,
    files_to_commit: Option<Vec<String>>,
    tag_name: Option<String>,
}

impl KernelData {
    fn set_last_version(&mut self, version: Version) {
        self.last_version = Some(version)
    }

    fn set_next_version(&mut self, version: semver::Version) {
        self.next_version = Some(version)
    }

    fn set_changelog(&mut self, changelog: String) {
        self.changelog = Some(changelog)
    }

    fn set_files_to_commit(&mut self, files: Vec<String>) {
        self.files_to_commit = Some(files);
    }

    fn set_tag_name(&mut self, tag_name: String) {
        self.tag_name = Some(tag_name);
    }

    fn require_last_version(&self) -> Result<&Version, failure::Error> {
        Ok(Self::_require("last_version", || {
            self.last_version.as_ref()
        })?)
    }

    fn require_next_version(&self) -> Result<&semver::Version, failure::Error> {
        Ok(Self::_require("next_version", || {
            self.next_version.as_ref()
        })?)
    }

    fn require_changelog(&self) -> Result<&str, failure::Error> {
        Ok(Self::_require("changelog", || self.changelog.as_ref())?)
    }

    fn require_files_to_commit(&self) -> Result<&[String], failure::Error> {
        Ok(Self::_require("files_to_commit", || {
            self.files_to_commit.as_ref()
        })?)
    }

    fn requite_tag_name(&self) -> Result<&str, failure::Error> {
        Ok(Self::_require("tag_name", || self.tag_name.as_ref())?)
    }

    fn _require<T>(
        desc: &'static str,
        query_fn: impl Fn() -> Option<T>,
    ) -> Result<T, failure::Error> {
        let data = query_fn().ok_or_else(|| KernelError::MissingRequiredData(desc))?;
        Ok(data)
    }
}

type KernelRoutineResult<T> = Result<T, failure::Error>;

trait KernelRoutine {
    fn execute(&self, kernel: &Kernel, data: &mut KernelData) -> KernelRoutineResult<()>;

    fn pre_flight(kernel: &Kernel, _data: &mut KernelData) -> KernelRoutineResult<()> {
        execute_request(|| kernel.dispatcher.pre_flight(), all_responses_into_result)?;
        Ok(())
    }

    fn get_last_release(kernel: &Kernel, data: &mut KernelData) -> KernelRoutineResult<()> {
        let (_, response) = kernel.dispatcher.get_last_release()?;
        let response = response.into_result()?;
        data.set_last_version(response);
        Ok(())
    }

    fn derive_next_version(kernel: &Kernel, data: &mut KernelData) -> KernelRoutineResult<()> {
        let responses = execute_request(
            || {
                kernel
                    .dispatcher
                    .derive_next_version(data.require_last_version()?.clone())
            },
            all_responses_into_result,
        )?;
        let next_version = responses
            .into_iter()
            .map(|(_, v)| v)
            .max()
            .expect("iterator from response map cannot be empty: this is a bug, aborting.");
        data.set_next_version(next_version);
        Ok(())
    }

    fn generate_notes(kernel: &Kernel, data: &mut KernelData) -> KernelRoutineResult<()> {
        let responses = execute_request(
            || {
                let params = request::GenerateNotesData {
                    start_rev: data.require_last_version()?.rev.clone(),
                    new_version: data.require_next_version()?.clone(),
                };
                kernel.dispatcher.generate_notes(params)
            },
            all_responses_into_result,
        )?;

        let changelog = responses.values().fold(String::new(), |mut summary, part| {
            summary.push_str(part);
            summary
        });

        log::info!("Would write the following changelog: ");
        log::info!("--------- BEGIN CHANGELOG ----------");
        log::info!("{}", changelog);
        log::info!("---------- END CHANGELOG -----------");

        data.set_changelog(changelog);

        Ok(())
    }

    fn prepare(kernel: &Kernel, data: &mut KernelData) -> KernelRoutineResult<()> {
        let responses = execute_request(
            || {
                kernel
                    .dispatcher
                    .prepare(data.require_next_version()?.clone())
            },
            all_responses_into_result,
        )?;

        let changed_files = responses
            .into_iter()
            .flat_map(|(_, v)| v.into_iter())
            .collect();

        data.set_files_to_commit(changed_files);

        Ok(())
    }

    fn verify_release(kernel: &Kernel, _data: &mut KernelData) -> KernelRoutineResult<()> {
        execute_request(
            || kernel.dispatcher.verify_release(),
            all_responses_into_result,
        )?;
        Ok(())
    }

    fn commit(kernel: &Kernel, data: &mut KernelData) -> KernelRoutineResult<()> {
        let params = request::CommitData {
            files_to_commit: data.require_files_to_commit()?.to_owned(),
            version: data.require_next_version()?.clone(),
            changelog: data.require_changelog()?.to_owned(),
        };

        let (_, response) = kernel.dispatcher.commit(params)?;

        let tag_name = response.into_result()?;

        data.set_tag_name(tag_name);

        Ok(())
    }

    fn publish(kernel: &Kernel, data: &mut KernelData) -> KernelRoutineResult<()> {
        execute_request(
            || {
                let params = request::PublishData {
                    tag_name: data.requite_tag_name()?.to_owned(),
                    changelog: data.require_changelog()?.to_owned(),
                };
                kernel.dispatcher.publish(params)
            },
            all_responses_into_result,
        )?;
        Ok(())
    }

    fn notify(kernel: &Kernel, _data: &mut KernelData) -> KernelRoutineResult<()> {
        execute_request(|| kernel.dispatcher.notify(()), all_responses_into_result)?;
        Ok(())
    }
}

impl KernelRoutine for PluginStep {
    fn execute(&self, kernel: &Kernel, data: &mut KernelData) -> KernelRoutineResult<()> {
        match self {
            PluginStep::PreFlight => PluginStep::pre_flight(kernel, data),
            PluginStep::GetLastRelease => PluginStep::get_last_release(kernel, data),
            PluginStep::DeriveNextVersion => PluginStep::derive_next_version(kernel, data),
            PluginStep::GenerateNotes => PluginStep::generate_notes(kernel, data),
            PluginStep::Prepare => PluginStep::prepare(kernel, data),
            PluginStep::VerifyRelease => PluginStep::verify_release(kernel, data),
            PluginStep::Commit => PluginStep::commit(kernel, data),
            PluginStep::Publish => PluginStep::publish(kernel, data),
            PluginStep::Notify => PluginStep::notify(kernel, data),
        }
    }
}

fn execute_request<RF, RFR, MF, MFR>(request_fn: RF, merge_fn: MF) -> Result<MFR, failure::Error>
where
    RF: Fn() -> Result<RFR, failure::Error>,
    MF: Fn(RFR) -> Result<MFR, failure::Error>,
{
    let response = request_fn()?;
    let merged = merge_fn(response)?;
    Ok(merged)
}

fn all_responses_into_result<T>(
    responses: Map<String, PluginResponse<T>>,
) -> Result<Map<String, T>, failure::Error> {
    responses
        .into_iter()
        .map(|(name, r)| {
            r.into_result()
                .map_err(|err| failure::format_err!("Plugin {:?} raised error: {}", name, err))
                .map(|data| (name, data))
        })
        .collect()
}
