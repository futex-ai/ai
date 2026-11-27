//! Shared MiniMax provider test support.

use std::sync::Arc;

use ai_interface::{ConversationMessage, ModelRequest, ModelSpec};
use ai_models_core::test_support::{
    RecordedRequests, fixtures_for_buffered_responses, recording_streaming_client,
};
use json_http::TransportBackedJsonHttpClient;
use json_http::{JsonHttpClient, JsonHttpResponse};
use unimock::Unimock;

use super::{CUSTOM_DEFAULTS, PROVIDER_ID};

/// Builds an unlisted MiniMax model spec with the provider defaults.
pub(super) fn custom_model_spec() -> ModelSpec {
    ModelSpec::custom(PROVIDER_ID, "minimax-custom", &CUSTOM_DEFAULTS)
}

pub(super) fn unused_http_client() -> Arc<dyn JsonHttpClient> {
    Arc::new(TransportBackedJsonHttpClient::new(Arc::new(Unimock::new(
        (),
    ))))
}

pub(super) fn recording_http_client(
    responses: impl IntoIterator<Item = JsonHttpResponse<serde_json::Value>>,
) -> (Arc<dyn JsonHttpClient>, RecordedRequests) {
    recording_streaming_client(fixtures_for_buffered_responses(
        responses.into_iter().collect(),
    ))
}

pub(super) fn simple_request() -> ModelRequest {
    ModelRequest {
        system_prompt: "system".to_owned(),
        messages: vec![ConversationMessage::user("hello")],
        tools: Vec::new(),
        response_schema: None,
        controls: Default::default(),
    }
}
