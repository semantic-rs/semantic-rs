use std::fmt::{self, Display};
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

pub type GitRevision = String;

pub type Null = ();

// TODO: consider using something more Markdown-oriented
pub type ReleaseNotes = String;

pub type MethodName = String;

pub type Warning = String;

pub type Error = String;

#[derive(Clone, Debug)]
pub enum Version {
    None(GitRevision),
    Semver(GitRevision, semver::Version),
}

impl Version {
    pub fn rev(&self) -> &str {
        match self {
            Version::None(rev) => &rev,
            Version::Semver(rev, _) => &rev,
        }
    }
}

pub mod request {
    use super::*;
    use crate::config::CfgMap;
    use std::collections::HashMap;

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

    pub type MethodsRequest = PluginRequest<MethodsRequestData>;
    pub type MethodsRequestData = Null;

    pub type PreFlightRequest = PluginRequest<PreFlightRequestData>;
    pub type PreFlightRequestData = Null;

    pub type GetLastReleaseRequest = PluginRequest<GetLastReleaseRequestData>;
    pub type GetLastReleaseRequestData = Null;

    pub type DeriveNextVersionRequest = PluginRequest<DeriveNextVersionRequestData>;
    pub type DeriveNextVersionRequestData = Version;

    pub type GenerateNotesRequest = PluginRequest<GenerateNotesRequestData>;

    #[derive(Clone, Debug)]
    pub struct GenerateNotesRequestData {
        pub start_rev: String,
        pub new_version: semver::Version,
    }

    pub type PrepareRequest = PluginRequest<PrepareRequestData>;
    pub type PrepareRequestData = Null;

    pub type VerifyReleaseRequest = PluginRequest<VerifyReleaseRequestData>;
    pub struct VerifyReleaseRequestData {
        version: Version,
    }

    pub type CommitRequest = PluginRequest<CommitRequestData>;
    pub type CommitRequestData = Null;

    pub type PublishRequest = PluginRequest<PublishRequestData>;
    pub type PublishRequestData = Null;

    pub type NotifyRequest = PluginRequest<NotifyRequestData>;
    pub type NotifyRequestData = Null;
}

pub mod response {
    use super::*;
    use crate::plugin::proto::request::{GenerateNotesRequestData, PrepareRequestData};
    use crate::plugin::PluginStep;
    use failure::Fail;
    use std::borrow::Borrow;
    use std::collections::HashMap;

    pub type PluginResult<T> = Result<PluginResponse<T>, failure::Error>;

    #[derive(Serialize, Deserialize, Clone, Debug)]
    pub struct PluginResponse<T> {
        warnings: Vec<Warning>,
        body: PluginResponseBody<T>,
    }

    #[derive(Serialize, Deserialize, Clone, Debug)]
    pub enum PluginResponseBody<T> {
        Error(Vec<Error>),
        Data(T),
    }

    impl<T> PluginResponse<T> {
        pub fn builder() -> PluginResponseBuilder<T> {
            PluginResponseBuilder::new()
        }

        pub fn into_result(self) -> Result<T, failure::Error> {
            self.warnings.iter().for_each(|w| log::warn!("{}", w));
            match self.body {
                PluginResponseBody::Error(errors) => {
                    let errors = errors.join("\n\t");
                    let mut error_msg = format!("\n\t{}", errors);
                    if error_msg.is_empty() {
                        error_msg = "<empty error message>".to_owned();
                    }
                    Err(failure::err_msg(error_msg))
                }
                PluginResponseBody::Data(data) => Ok(data),
            }
        }
    }

    pub struct PluginResponseBuilder<T> {
        warnings: Vec<Warning>,
        errors: Vec<Error>,
        data: Option<T>,
    }

    impl<T> PluginResponseBuilder<T> {
        pub fn new() -> Self {
            PluginResponseBuilder {
                warnings: vec![],
                errors: vec![],
                data: None,
            }
        }

        pub fn warning<W: Into<Warning>>(&mut self, new: W) -> &mut Self {
            self.warnings.push(new.into());
            self
        }

        pub fn warnings(&mut self, new: &[&str]) -> &mut Self {
            let new_warnings = new.iter().map(|&w| String::from(w));
            self.warnings.extend(new_warnings);
            self
        }

        pub fn error<E: Into<failure::Error>>(&mut self, new: E) -> &mut Self {
            self.errors.push(format!("{}", new.into()));
            self
        }

        pub fn errors<'a, E>(&mut self, new: &'a [E]) -> &mut Self
        where
            failure::Error: From<&'a E>,
        {
            let new_errors = new
                .iter()
                .map(failure::Error::from)
                .map(|err| format!("{}", err));
            self.errors.extend(new_errors);
            self
        }

        pub fn body<IT: Into<T>>(&mut self, data: IT) -> &mut Self {
            self.data = Some(data.into());
            self
        }

        pub fn build(&mut self) -> PluginResponse<T> {
            use std::mem;

            let warnings = mem::replace(&mut self.warnings, Vec::new());
            let errors = mem::replace(&mut self.errors, Vec::new());
            let data = self.data.take();

            let body = if errors.is_empty() {
                let data =
                    data.expect("data must be present in response if it's a successful response");
                PluginResponseBody::Data(data)
            } else {
                PluginResponseBody::Error(errors)
            };

            PluginResponse { warnings, body }
        }
    }

    pub type MethodsResponse = HashMap<PluginStep, bool>;

    pub type PreFlightResponse = Null;

    pub type GetLastReleaseResponse = Version;

    pub type DeriveNextVersionResponse = semver::Version;

    pub type GenerateNotesResponse = ReleaseNotes;

    pub type PrepareResponse = Null;

    pub type VerifyReleaseResponse = Null;

    pub type CommitResponse = Null;

    pub type PublishResponse = Null;

    pub type NotifyResponse = Null;
}
