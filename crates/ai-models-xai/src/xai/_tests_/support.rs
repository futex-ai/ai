//! Shared xAI provider test helpers.

use std::sync::Arc;

use ai_models_core::test_support::{
    RecordedRequests, fixtures_for_buffered_responses, recording_streaming_client,
};
use json_http::{JsonHttpClient, JsonHttpResponse, TransportBackedJsonHttpClient};
use serde_json::Value;
use unimock::Unimock;

pub(super) fn recording_http_client(
    response: JsonHttpResponse<Value>,
) -> (Arc<dyn JsonHttpClient>, RecordedRequests) {
    recording_http_client_responses(vec![response])
}

pub(super) fn recording_http_client_responses(
    responses: Vec<JsonHttpResponse<Value>>,
) -> (Arc<dyn JsonHttpClient>, RecordedRequests) {
    recording_streaming_client(fixtures_for_buffered_responses(responses))
}

pub(super) fn unused_http_client() -> Arc<dyn JsonHttpClient> {
    Arc::new(TransportBackedJsonHttpClient::new(Arc::new(Unimock::new(
        (),
    ))))
}
