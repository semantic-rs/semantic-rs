use std::collections::HashMap;

use crate::plugin::proto::request::{MethodsRequest, PluginRequest};
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
}
