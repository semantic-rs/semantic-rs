use super::proto::{
    request::{
        CommitRequest, DeriveNextVersionRequest, GenerateNotesRequest, GetLastReleaseRequest,
        MethodsRequest, NotifyRequest, PreFlightRequest, PrepareRequest, PublishRequest,
        VerifyReleaseRequest,
    },
    response::{
        CommitResponse, DeriveNextVersionResponse, GenerateNotesResponse, GetLastReleaseResponse,
        MethodsResponse, NotifyResponse, PluginResponse, PluginResult, PreFlightResponse,
        PrepareResponse, PublishResponse, VerifyReleaseResponse,
    },
};

pub trait PluginInterface {
    fn methods(&self, _req: MethodsRequest) -> PluginResult<MethodsResponse> {
        Ok(PluginResponse::builder()
            .warning("default methods() implementation called: returning an empty map")
            .body(MethodsResponse::default())
            .build())
    }

    fn pre_flight(&self, _params: PreFlightRequest) -> PluginResult<PreFlightResponse> {
        not_implemented_response()
    }

    fn get_last_release(
        &self,
        _params: GetLastReleaseRequest,
    ) -> PluginResult<GetLastReleaseResponse> {
        not_implemented_response()
    }

    fn derive_next_version(
        &self,
        _params: DeriveNextVersionRequest,
    ) -> PluginResult<DeriveNextVersionResponse> {
        not_implemented_response()
    }

    fn generate_notes(&self, _params: GenerateNotesRequest) -> PluginResult<GenerateNotesResponse> {
        not_implemented_response()
    }

    fn prepare(&self, _params: PrepareRequest) -> PluginResult<PrepareResponse> {
        not_implemented_response()
    }

    fn verify_release(&self, _params: VerifyReleaseRequest) -> PluginResult<VerifyReleaseResponse> {
        not_implemented_response()
    }

    fn commit(&self, _params: CommitRequest) -> PluginResult<CommitResponse> {
        not_implemented_response()
    }

    fn publish(&self, _params: PublishRequest) -> PluginResult<PublishResponse> {
        not_implemented_response()
    }

    fn notify(&self, _params: NotifyRequest) -> PluginResult<NotifyResponse> {
        not_implemented_response()
    }
}

fn not_implemented_response<T>() -> PluginResult<T> {
    Ok(PluginResponse::builder()
        .error(failure::err_msg("method not implemented"))
        .build())
}
