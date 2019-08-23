pub mod response;

use serde::{Deserialize, Serialize};

pub type GitRevision = String;

pub type Warning = String;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Version {
    pub rev: GitRevision,
    pub semver: Option<semver::Version>,
}
