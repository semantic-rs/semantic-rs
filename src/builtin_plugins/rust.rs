use std::collections::HashMap;
use std::ops::Try;

use failure::Fail;

use crate::config::{CfgMap, CfgMapExt};
use crate::plugin::proto::{
    request,
    response::{self, PluginResponse},
    GitRevision, Version,
};
use crate::plugin::{PluginInterface, PluginStep};

pub struct RustPlugin {}

impl RustPlugin {
    pub fn new() -> Self {
        RustPlugin {}
    }
}

impl PluginInterface for RustPlugin {
    fn methods(&self, req: request::Methods) -> response::Methods {
        let mut methods = HashMap::new();
        methods.insert(PluginStep::PreFlight, true);
        methods.insert(PluginStep::Prepare, true);
        methods.insert(PluginStep::VerifyRelease, true);
        PluginResponse::from_ok(methods)
    }

    fn pre_flight(&self, params: request::PreFlight) -> response::PreFlight {
        let mut response = PluginResponse::builder();
        if !params.env.contains_key("CARGO_TOKEN") {
            response.error(RustPluginError::TokenUndefined);
        }
        response.body(()).build()
    }
}

#[derive(Fail, Debug)]
pub enum RustPluginError {
    #[fail(display = "the CARGO_TOKEN environment variable is not configured")]
    TokenUndefined,
}
