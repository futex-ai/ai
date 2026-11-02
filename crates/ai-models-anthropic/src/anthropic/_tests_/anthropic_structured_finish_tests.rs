//! Tests for structured-output terminal finish handling.

use ai_interface::{
    ConversationMessage, FinishReason, Model, ModelRequest, ModelResponse, StructuredOutputSchema,
};
use serde_json::json;

use crate::CLAUDE_SONNET_4_6;

use super::{AnthropicModel, stream_support::client_for_buffered_bodies};

#[tokio::test]
async fn structured_output_refusal_preserves_filtered_finish_reason() {
    let response = parse_fixture(
        json!({
            "stop_reason": "refusal",
            "content": [
                { "type": "text", "text": "I cannot provide that." }
            ]
        }),
        Some(&status_schema()),
    )
    .await
    .expect("filtered structured response should parse");

    assert_eq!(response.finish_reason, FinishReason::Filtered);
    assert_eq!(response.structured_output, None);
    assert_eq!(response.text(), "I cannot provide that.");
}

#[tokio::test]
async fn structured_output_truncation_preserves_truncated_finish_reason() {
    let response = parse_fixture(
        json!({
            "stop_reason": "max_tokens",
            "content": [
                { "type": "text", "text": "{\"summary\":" }
            ]
        }),
        Some(&status_schema()),
    )
    .await
    .expect("truncated structured response should parse");

    assert_eq!(response.finish_reason, FinishReason::Truncated);
    assert_eq!(response.structured_output, None);
    assert_eq!(response.text(), "{\"summary\":");
}

#[tokio::test]
async fn terminal_response_with_tool_use_suppresses_tool_calls() {
    let response = parse_fixture(
        json!({
            "stop_reason": "max_tokens",
            "content": [{
                "type": "tool_use",
                "id": "call_1",
                "name": "memory_read",
                "input": { "path": "root" }
            }],
            "usage": {
                "input_tokens": 8,
                "output_tokens": 4
            }
        }),
        None,
    )
    .await
    .expect("terminal Anthropic response should parse");

    assert_eq!(response.finish_reason, FinishReason::Truncated);
    assert!(response.tool_calls().is_empty());
}

fn status_schema() -> StructuredOutputSchema {
    StructuredOutputSchema {
        name: "status".to_owned(),
        schema: json!({
            "type": "object",
            "properties": {
                "summary": {"type": "string"},
                "done": {"type": "boolean"}
            },
            "required": ["summary", "done"]
        }),
    }
}

async fn parse_fixture(
    body: serde_json::Value,
    schema: Option<&StructuredOutputSchema>,
) -> ai_interface::ModelResult<ModelResponse> {
    let (http_client, _) = client_for_buffered_bodies(vec![body]);
    let model = AnthropicModel::new(http_client, CLAUDE_SONNET_4_6, "key");
    model
        .complete(&ModelRequest {
            system_prompt: "system".to_owned(),
            messages: vec![ConversationMessage::user("hello")],
            tools: Vec::new(),
            response_schema: schema.cloned(),
            controls: Default::default(),
        })
        .await
}
