use std::collections::HashMap;

use super::{Null, Version};
use crate::config::CfgMap;

pub struct PluginRequest<T> {
    pub cfg_map: CfgMap,
    pub env: HashMap<String, String>,
    pub data: T,
}

impl<T> PluginRequest<T> {
    pub fn new(cfg_map: CfgMap, data: T) -> Self {
        Self::with_env(cfg_map, std::env::vars().collect(), data)
    }

    pub fn with_env(cfg_map: CfgMap, env: HashMap<String, String>, data: T) -> Self {
        PluginRequest { cfg_map, env, data }
    }
}

impl<T: Default> PluginRequest<T> {
    pub fn with_default_data(cfg_map: CfgMap) -> Self {
        PluginRequest::new(cfg_map, Default::default())
    }
}

pub type Methods = PluginRequest<MethodsData>;
pub type MethodsData = Null;

pub type PreFlight = PluginRequest<PreFlightData>;
pub type PreFlightData = Null;

pub type GetLastRelease = PluginRequest<GetLastReleaseData>;
pub type GetLastReleaseData = Null;

pub type DeriveNextVersion = PluginRequest<DeriveNextVersionData>;
pub type DeriveNextVersionData = Version;

pub type GenerateNotes = PluginRequest<GenerateNotesData>;

#[derive(Clone, Debug)]
pub struct GenerateNotesData {
    pub start_rev: String,
    pub new_version: semver::Version,
}

pub type Prepare = PluginRequest<PrepareData>;
pub type PrepareData = semver::Version;

pub type VerifyRelease = PluginRequest<VerifyReleaseData>;
pub type VerifyReleaseData = Null;

pub type Commit = PluginRequest<CommitData>;

#[derive(Clone, Debug)]
pub struct CommitData {
    pub files_to_commit: Vec<String>,
    pub version: semver::Version,
    pub changelog: String,
}

pub type Publish = PluginRequest<PublishData>;

#[derive(Clone, Debug)]
pub struct PublishData {
    pub tag_name: String,
    pub changelog: String,
}

pub type Notify = PluginRequest<NotifyData>;
pub type NotifyData = Null;
