use std::collections::HashMap;

use failure::Fail;

use crate::plugin::proto::request::{MethodsRequest, PluginRequest, PreFlightRequest};
use crate::plugin::proto::response::{
    MethodsResponse, PluginResponse, PluginResult, PreFlightResponse,
};
use crate::plugin::{PluginInterface, PluginStep};

pub struct GithubPlugin {}

impl GithubPlugin {
    pub fn new() -> Self {
        GithubPlugin {}
    }
}

impl PluginInterface for GithubPlugin {
    fn methods(&self, req: MethodsRequest) -> PluginResult<MethodsResponse> {
        let mut methods = HashMap::new();
        methods.insert(PluginStep::PreFlight, true);
        methods.insert(PluginStep::Publish, true);
        let resp = PluginResponse::builder().body(methods).build();
        Ok(resp)
    }

    fn pre_flight(&self, params: PreFlightRequest) -> PluginResult<PreFlightResponse> {
        let mut response = PluginResponse::builder();
        if !params.env.contains_key("GH_TOKEN") {
            response.error(GithubPluginError::TokenUndefined);
        }
        Ok(response.body(()).build())
    }
}

#[derive(Fail, Debug)]
pub enum GithubPluginError {
    #[fail(display = "the GH_TOKEN environment variable is not configured")]
    TokenUndefined,
}
