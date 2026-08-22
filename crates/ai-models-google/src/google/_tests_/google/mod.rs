//! Tests for Google request mapping and response parsing.

use std::sync::Arc;

use ai_interface::{ConversationMessage, ModelRequest};
use json_http::{JsonHttpClient, JsonHttpResponse};
use serde_json::json;

use super::GoogleModel;

mod controls_tests;
mod finish_reason_tests;
mod multimodal_tests;
mod operation_id_tests;
mod request_tests;
mod stream_error_tests;
mod stream_support;
mod streaming_tests;
mod structured_finish_tests;
mod thinking_tests;

fn recording_http_client(
    response: JsonHttpResponse<serde_json::Value>,
) -> (Arc<dyn JsonHttpClient>, stream_support::RecordedRequests) {
    assert_eq!(response.status, 200, "stream fixture must be successful");
    stream_support::recording_streaming_client(vec![vec![stream_support::event(response.body)]])
}

fn simple_request() -> ModelRequest {
    ModelRequest {
        system_prompt: "system".to_owned(),
        messages: vec![ConversationMessage::user("hello")],
        tools: Vec::new(),
        response_schema: None,
        controls: Default::default(),
    }
}

fn google_response_body(raw_reason: Option<&str>, include_tool: bool) -> serde_json::Value {
    let parts = if include_tool {
        json!([{
            "functionCall": {
                "id": "call_1",
                "name": "memory_read",
                "args": { "path": "root" }
            }
        }])
    } else {
        json!([{ "text": "Done" }])
    };
    let mut candidate = json!({
        "content": {
            "parts": parts
        }
    });
    if let Some(raw_reason) = raw_reason {
        candidate["finishReason"] = json!(raw_reason);
    }
    json!({
        "candidates": [candidate]
    })
}
