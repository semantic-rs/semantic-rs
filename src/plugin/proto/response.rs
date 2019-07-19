use std::ops::Try;

use serde::{Deserialize, Serialize};

use super::{Error, Null, ReleaseNotes, Version, Warning};
use crate::plugin::PluginStep;

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
}

impl<T> Try for PluginResponse<T> {
    type Ok = T;
    type Error = failure::Error;

    fn into_result(self) -> Result<Self::Ok, Self::Error> {
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

    fn from_error(v: Self::Error) -> Self {
        PluginResponse {
            warnings: vec![],
            body: PluginResponseBody::Error(vec![format!("{}", v)]),
        }
    }

    fn from_ok(v: Self::Ok) -> Self {
        PluginResponse {
            warnings: vec![],
            body: PluginResponseBody::Data(v),
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

pub type Name = PluginResponse<String>;

pub type Methods = PluginResponse<MethodsData>;
pub type MethodsData = Vec<PluginStep>;

pub type PreFlight = PluginResponse<PreFlightData>;
pub type PreFlightData = Null;

pub type GetLastRelease = PluginResponse<GetLastReleaseData>;
pub type GetLastReleaseData = Version;

pub type DeriveNextVersion = PluginResponse<DeriveNextVersionData>;
pub type DeriveNextVersionData = semver::Version;

pub type GenerateNotes = PluginResponse<GenerateNotesData>;
pub type GenerateNotesData = ReleaseNotes;

pub type Prepare = PluginResponse<PrepareData>;
/// List of changed files to be committed
pub type PrepareData = Vec<String>;

pub type VerifyRelease = PluginResponse<VerifyReleaseData>;
pub type VerifyReleaseData = Null;

pub type Commit = PluginResponse<CommitData>;
/// Name of the created git tag
pub type CommitData = String;

pub type Publish = PluginResponse<PublishData>;
pub type PublishData = Null;

pub type Notify = PluginResponse<NotifyData>;
pub type NotifyData = Null;
