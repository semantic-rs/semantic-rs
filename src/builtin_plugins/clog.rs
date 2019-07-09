use std::collections::HashMap;

use crate::plugin::proto::request::{MethodsRequest, PluginRequest};
use crate::plugin::proto::response::{
    MethodsResponse, PluginResponse, PluginResult, PreFlightResponse,
};
use crate::plugin::{PluginInterface, PluginStep};

pub struct ClogPlugin {}

impl ClogPlugin {
    pub fn new() -> Self {
        ClogPlugin {}
    }
}

impl PluginInterface for ClogPlugin {
    fn methods(&self, req: MethodsRequest) -> PluginResult<MethodsResponse> {
        let mut methods = HashMap::new();
        methods.insert(PluginStep::DeriveNextVersion, true);
        methods.insert(PluginStep::GenerateNotes, true);
        let resp = PluginResponse::builder().body(methods).build();
        Ok(resp)
    }
}
