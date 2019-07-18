use std::collections::HashMap;
use std::ops::Try;

use failure::Fail;

use crate::plugin::proto::{
    request,
    response::{self, PluginResponse},
};
use crate::plugin::{PluginInterface, PluginStep};

pub struct GithubPlugin {}

impl GithubPlugin {
    pub fn new() -> Self {
        GithubPlugin {}
    }
}

impl PluginInterface for GithubPlugin {
    fn methods(&self, _req: request::Methods) -> response::Methods {
        let mut methods = HashMap::new();
        methods.insert(PluginStep::PreFlight, true);
        methods.insert(PluginStep::Publish, true);
        PluginResponse::from_ok(methods)
    }

    fn pre_flight(&self, params: request::PreFlight) -> response::PreFlight {
        let mut response = PluginResponse::builder();
        if !params.env.contains_key("GH_TOKEN") {
            response.error(GithubPluginError::TokenUndefined);
        }
        response.body(()).build()
    }
}

#[derive(Fail, Debug)]
pub enum GithubPluginError {
    #[fail(display = "the GH_TOKEN environment variable is not configured")]
    TokenUndefined,
}
