//! Anthropic stream failure classification tests.

use ai_interface::{ConversationMessage, Model, ModelError, ModelRequest};
use serde_json::json;

use super::AnthropicModel;
use crate::anthropic::stream_support::{event, recording_streaming_client};

#[tokio::test]
async fn classifies_first_provider_error_without_partial_generation() {
    let error_event = event(
        "error",
        json!({
            "type": "error",
            "error": {"type": "overloaded_error", "message": "Overloaded"}
        }),
    );
    let (http_client, _) = recording_streaming_client(vec![vec![error_event]]);
    let model = AnthropicModel::new(http_client, "claude-sonnet-4-6", "key");

    let error = model
        .complete(&simple_request())
        .await
        .expect_err("overloaded stream should fail");

    assert!(matches!(error, ModelError::TransientProvider { .. }));
}

#[tokio::test]
async fn preserves_first_rate_limit_event_as_rate_limited() {
    let error_event = event(
        "error",
        json!({
            "type": "error",
            "error": {"type": "rate_limit_error", "message": "Slow down"}
        }),
    );
    let (http_client, _) = recording_streaming_client(vec![vec![error_event]]);
    let model = AnthropicModel::new(http_client, "claude-sonnet-4-6", "key");

    let error = model
        .complete(&simple_request())
        .await
        .expect_err("rate-limited stream should fail");

    assert!(matches!(error, ModelError::RateLimited { .. }));
}

#[tokio::test]
async fn classifies_provider_error_after_progress_as_interrupted() {
    let events = vec![
        message_start(),
        event(
            "error",
            json!({
                "type": "error",
                "error": {"type": "overloaded_error", "message": "Overloaded"}
            }),
        ),
    ];
    let (http_client, _) = recording_streaming_client(vec![events]);
    let model = AnthropicModel::new(http_client, "claude-sonnet-4-6", "key");

    let error = model
        .complete(&simple_request())
        .await
        .expect_err("partially generated stream should fail");

    assert!(matches!(error, ModelError::Interrupted { .. }));
}

#[tokio::test]
async fn classifies_eof_before_and_after_progress() {
    let (http_client, _) = recording_streaming_client(vec![vec![], vec![message_start()]]);
    let model = AnthropicModel::new(http_client, "claude-sonnet-4-6", "key");

    let before = model
        .complete(&simple_request())
        .await
        .expect_err("empty stream should fail");
    let after = model
        .complete(&simple_request())
        .await
        .expect_err("partial stream should fail");

    assert!(matches!(before, ModelError::TransientProvider { .. }));
    assert!(matches!(after, ModelError::Interrupted { .. }));
}

#[tokio::test]
async fn classifies_transport_failure_before_and_after_progress() {
    let (http_client, _) = recording_streaming_client(vec![
        vec![Err(json_http::Error::transport("offline"))],
        vec![message_start(), Err(json_http::Error::transport("reset"))],
    ]);
    let model = AnthropicModel::new(http_client, "claude-sonnet-4-6", "key");

    let before = model
        .complete(&simple_request())
        .await
        .expect_err("opening transport failure should fail");
    let after = model
        .complete(&simple_request())
        .await
        .expect_err("mid-stream transport failure should fail");

    assert!(matches!(before, ModelError::TransientProvider { .. }));
    assert!(matches!(after, ModelError::Interrupted { .. }));
}

#[tokio::test]
async fn invalid_fragmented_tool_json_is_an_interruption() {
    let events = vec![
        message_start(),
        event(
            "content_block_start",
            json!({
                "type": "content_block_start",
                "index": 0,
                "content_block": {
                    "type": "tool_use",
                    "id": "call_1",
                    "name": "memory_read",
                    "input": {}
                }
            }),
        ),
        event(
            "content_block_delta",
            json!({
                "type": "content_block_delta",
                "index": 0,
                "delta": {"type": "input_json_delta", "partial_json": "{"}
            }),
        ),
        event(
            "content_block_stop",
            json!({"type": "content_block_stop", "index": 0}),
        ),
    ];
    let (http_client, _) = recording_streaming_client(vec![events]);
    let model = AnthropicModel::new(http_client, "claude-sonnet-4-6", "key");

    let error = model
        .complete(&simple_request())
        .await
        .expect_err("invalid partial JSON should fail");

    assert!(matches!(error, ModelError::Interrupted { .. }));
}

fn message_start() -> crate::anthropic::stream_support::StreamItem {
    event(
        "message_start",
        json!({
            "type": "message_start",
            "message": {
                "content": [],
                "usage": {"input_tokens": 1, "output_tokens": 0}
            }
        }),
    )
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
