use std::ops::Try;

use super::Warning;
use crate::plugin_support::flow::ProvisionCapability;
use crate::plugin_support::PluginStep;

#[derive(Debug)]
pub struct PluginResponse<T> {
    warnings: Vec<Warning>,
    body: PluginResponseBody<T>,
}

#[derive(Debug)]
pub enum PluginResponseBody<T> {
    Error(failure::Error),
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
            PluginResponseBody::Error(err) => Err(err),
            PluginResponseBody::Data(data) => Ok(data),
        }
    }

    fn from_error(v: Self::Error) -> Self {
        PluginResponse {
            warnings: vec![],
            body: PluginResponseBody::Error(v),
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
    error: Option<failure::Error>,
    data: Option<T>,
}

impl<T> PluginResponseBuilder<T> {
    pub fn new() -> Self {
        PluginResponseBuilder {
            warnings: vec![],
            error: None,
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

    pub fn error<E: Into<failure::Error>>(&mut self, err: E) -> PluginResponse<T> {
        self.error = Some(err.into());
        self.build()
    }

    pub fn body<IT: Into<T>>(&mut self, data: IT) -> PluginResponse<T> {
        self.data = Some(data.into());
        self.build()
    }

    pub fn build(&mut self) -> PluginResponse<T> {
        use std::mem;

        let warnings = mem::replace(&mut self.warnings, Vec::new());
        let error = self.error.take();
        let data = self.data.take();

        let body = if let Some(err) = error {
            PluginResponseBody::Error(err)
        } else {
            let data = data.expect("data must be present in response if it's a successful response");
            PluginResponseBody::Data(data)
        };

        PluginResponse { warnings, body }
    }
}

pub type Null = PluginResponse<()>;

pub type Name = PluginResponse<String>;

pub type ProvisionCapabilities = PluginResponse<Vec<ProvisionCapability>>;

pub type GetValue = PluginResponse<serde_json::Value>;

pub type Config = PluginResponse<serde_json::Value>;

pub type Methods = PluginResponse<MethodsData>;
pub type MethodsData = Vec<PluginStep>;
