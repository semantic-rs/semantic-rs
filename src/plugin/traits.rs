use std::ops::Try;

use super::proto::{
    request,
    response::{self, PluginResponse},
};

pub trait PluginInterface {
    fn methods(&self, _req: request::Methods) -> response::Methods {
        PluginResponse::builder()
            .warning("default methods() implementation called: returning an empty map")
            .body(response::MethodsData::default())
            .build()
    }

    fn pre_flight(&self, _params: request::PreFlight) -> response::PreFlight {
        not_implemented_response()
    }

    fn get_last_release(&self, _params: request::GetLastRelease) -> response::GetLastRelease {
        not_implemented_response()
    }

    fn derive_next_version(
        &self,
        _params: request::DeriveNextVersion,
    ) -> response::DeriveNextVersion {
        not_implemented_response()
    }

    fn generate_notes(&self, _params: request::GenerateNotes) -> response::GenerateNotes {
        not_implemented_response()
    }

    fn prepare(&self, _params: request::Prepare) -> response::Prepare {
        not_implemented_response()
    }

    fn verify_release(&self, _params: request::VerifyRelease) -> response::VerifyRelease {
        not_implemented_response()
    }

    fn commit(&self, _params: request::Commit) -> response::Commit {
        not_implemented_response()
    }

    fn publish(&self, _params: request::Publish) -> response::Publish {
        not_implemented_response()
    }

    fn notify(&self, _params: request::Notify) -> response::Notify {
        not_implemented_response()
    }
}

fn not_implemented_response<T>() -> PluginResponse<T> {
    PluginResponse::from_error(failure::err_msg("method not implemented"))
}
