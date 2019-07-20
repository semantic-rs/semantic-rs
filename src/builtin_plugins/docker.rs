use std::ops::Try;

use failure::Fail;

use crate::config::CfgMapExt;
use crate::plugin::proto::{
    request,
    response::{self, PluginResponse},
};
use crate::plugin::{PluginInterface, PluginStep};

#[derive(Default)]
pub struct DockerPlugin {
    cfg: Option<Config>,
    state: Option<State>,
}

impl DockerPlugin {
    pub fn new() -> Self {
        DockerPlugin::default()
    }
}

struct Config {

}

struct State {

}

impl PluginInterface for DockerPlugin {
    fn name(&self) -> response::Name {
        PluginResponse::from_ok("docker".into())
    }

    fn methods(&self, req: request::Methods) -> response::Methods {
        PluginResponse::from_ok(vec![
            PluginStep::PreFlight,
            PluginStep::Prepare,
            PluginStep::VerifyRelease,
            PluginStep::Publish,
        ])
    }

    fn pre_flight(&mut self, req: request::PreFlight) -> response::PreFlight {
        unimplemented!()
    }

    fn prepare(&mut self, req: request::Prepare) -> response::Prepare {
        unimplemented!()
    }

    fn verify_release(&mut self, req: request::VerifyRelease) -> response::VerifyRelease {
        unimplemented!()
    }

    fn publish(&mut self, req: request::Publish) -> response::Publish {
        unimplemented!()
    }
}

#[derive(Fail, Debug)]
enum DockerPluginError {
    #[fail(display = "docker repo credentials are not defined, cannot push the image")]
    CredentialsUndefined,
}