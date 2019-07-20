use std::collections::HashMap;

use super::{Null, Version};
use crate::config::CfgMap;

pub struct PluginRequest<'a, T> {
    pub cfg_map: &'a CfgMap,
    pub env: &'a HashMap<String, String>,
    pub data: &'a T,
}

impl<'a, T: 'a> PluginRequest<'a, T> {
    pub fn new(cfg_map: &'a CfgMap, env: &'a HashMap<String, String>, data: &'a T) -> Self {
        Self::with_env(cfg_map, env, data)
    }

    pub fn with_env(cfg_map: &'a CfgMap, env: &'a HashMap<String, String>, data: &'a T) -> Self {
        PluginRequest { cfg_map, env, data }
    }
}

impl<'a> PluginRequest<'a, ()> {
    pub fn new_null(cfg_map: &'a CfgMap, env: &'a HashMap<String, String>) -> Self {
        PluginRequest::new(cfg_map, env, &())
    }
}

pub type Methods<'a> = PluginRequest<'a, MethodsData>;
pub type MethodsData = Null;

pub type PreFlight<'a> = PluginRequest<'a, PreFlightData>;
pub type PreFlightData = Null;

pub type GetLastRelease<'a> = PluginRequest<'a, GetLastReleaseData>;
pub type GetLastReleaseData = Null;

pub type DeriveNextVersion<'a> = PluginRequest<'a, DeriveNextVersionData>;
pub type DeriveNextVersionData = Version;

pub type GenerateNotes<'a> = PluginRequest<'a, GenerateNotesData>;

#[derive(Clone, Debug)]
pub struct GenerateNotesData {
    pub start_rev: String,
    pub new_version: semver::Version,
}

pub type Prepare<'a> = PluginRequest<'a, PrepareData>;
pub type PrepareData = semver::Version;

pub type VerifyRelease<'a> = PluginRequest<'a, VerifyReleaseData>;
pub type VerifyReleaseData = Null;

pub type Commit<'a> = PluginRequest<'a, CommitData>;

#[derive(Clone, Debug)]
pub struct CommitData {
    pub files_to_commit: Vec<String>,
    pub version: semver::Version,
    pub changelog: String,
}

pub type Publish<'a> = PluginRequest<'a, PublishData>;

#[derive(Clone, Debug)]
pub struct PublishData {
    pub tag_name: String,
    pub changelog: String,
}

pub type Notify<'a> = PluginRequest<'a, NotifyData>;
pub type NotifyData = Null;
