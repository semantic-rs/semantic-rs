use std::collections::HashMap;

use failure::Fail;

use crate::plugin::proto::request::{MethodsRequest, PluginRequest, PreFlightRequest};
use crate::plugin::proto::response::{
    MethodsResponse, PluginResponse, PluginResult, PreFlightResponse,
};
use crate::plugin::{PluginInterface, PluginStep};

pub struct RustPlugin {}

impl RustPlugin {
    pub fn new() -> Self {
        RustPlugin {}
    }
}

impl PluginInterface for RustPlugin {
    fn methods(&self, req: MethodsRequest) -> PluginResult<MethodsResponse> {
        let mut methods = HashMap::new();
        methods.insert(PluginStep::PreFlight, true);
        methods.insert(PluginStep::Prepare, true);
        methods.insert(PluginStep::VerifyRelease, true);
        let resp = PluginResponse::builder().body(methods).build();
        Ok(resp)
    }

    fn pre_flight(&self, params: PreFlightRequest) -> PluginResult<PreFlightResponse> {
        let mut response = PluginResponse::builder();
        if !params.env.contains_key("CARGO_TOKEN") {
            response.error(RustPluginError::TokenUndefined);
        }
        Ok(response.body(()).build())
    }
}

#[derive(Fail, Debug)]
pub enum RustPluginError {
    #[fail(display = "the CARGO_TOKEN environment variable is not configured")]
    TokenUndefined,
}

