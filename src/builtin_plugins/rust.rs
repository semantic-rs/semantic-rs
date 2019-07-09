use std::collections::HashMap;

use crate::plugin::proto::request::{MethodsRequest, PluginRequest};
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
}
