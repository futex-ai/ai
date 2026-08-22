//! Shared Kimi provider test helpers.

use std::sync::Arc;

use ai_interface::{ConversationMessage, ModelRequest};
use ai_models_core::test_support::{
    RecordedRequests, SseFixture, fixtures_for_buffered_responses, recording_streaming_client,
};
use json_http::{JsonHttpClient, JsonHttpResponse, TransportBackedJsonHttpClient};
use serde_json::{Value, json};
use unimock::Unimock;

pub(super) fn unused_http_client() -> Arc<dyn JsonHttpClient> {
    Arc::new(TransportBackedJsonHttpClient::new(Arc::new(Unimock::new(
        (),
    ))))
}

pub(super) fn recording_http_client(
    response: JsonHttpResponse<Value>,
) -> (Arc<dyn JsonHttpClient>, RecordedRequests) {
    recording_streaming_client(fixtures_for_buffered_responses(vec![response]))
}

pub(super) fn transport_failure_http_client(message: &'static str) -> Arc<dyn JsonHttpClient> {
    recording_streaming_client(vec![SseFixture::Stream(vec![Err(
        json_http::Error::transport(message),
    )])])
    .0
}

pub(super) fn successful_response(content: Option<&str>) -> JsonHttpResponse<Value> {
    JsonHttpResponse {
        status: 200,
        body: json!({
            "choices": [{
                "finish_reason": "stop",
                "message": {
                    "content": content,
                    "reasoning_content": "private reasoning"
                }
            }]
        }),
    }
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
