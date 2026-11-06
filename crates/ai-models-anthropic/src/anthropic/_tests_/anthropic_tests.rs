//! Tests for Anthropic request mapping and response parsing.

use std::sync::Arc;

use ai_interface::{
    ConversationMessage, FinishReason, Model, ModelRequest, StructuredOutputSchema, ToolDefinition,
};
use json_http::{JsonHttpClient, JsonHttpResponse};
use serde_json::json;

use crate::CLAUDE_SONNET_4_6;
use crate::anthropic::stream_support::{RecordedRequests, client_for_buffered_bodies};

use super::AnthropicModel;

#[tokio::test]
async fn builds_anthropic_tool_requests_and_parses_response() {
    let (http_client, requests) = recording_http_client(JsonHttpResponse {
        headers: Default::default(),
        status: 200,
        body: json!({
            "stop_reason": "tool_use",
            "content": [
                { "type": "text", "text": "Done" },
                {
                    "type": "tool_use",
                    "id": "call_1",
                    "name": "memory_read",
                    "input": { "path": "root" }
                }
            ],
            "usage": {
                "input_tokens": 120,
                "cache_creation_input_tokens": 10,
                "cache_read_input_tokens": 40,
                "output_tokens": 32
            }
        }),
    });
    let model = AnthropicModel::new(http_client, CLAUDE_SONNET_4_6, "anthropic-key");

    let response = model
        .complete(&ModelRequest {
            system_prompt: "system".to_owned(),
            messages: vec![ConversationMessage::user("hello")],
            tools: vec![ToolDefinition {
                name: "memory_read".to_owned(),
                description: "Read memory".to_owned(),
                input_schema: json!({
                    "type": "object",
                    "properties": {
                        "path": {"type": "string"}
                    },
                    "required": ["path"]
                }),
                activity_verb: Some("Remembering".to_owned()),
            }],
            response_schema: None,
            controls: Default::default(),
        })
        .await
        .expect("Anthropic response should parse");

    let requests = requests
        .lock()
        .expect("requests lock should not be poisoned");
    assert_eq!(requests[0].url, "https://api.anthropic.com/v1/messages");
    assert_eq!(
        requests[0].headers.get("x-api-key"),
        Some(&"anthropic-key".to_owned())
    );
    assert_eq!(
        requests[0].headers.get("anthropic-version"),
        Some(&"2023-06-01".to_owned())
    );
    assert_eq!(
        serde_json::from_slice::<serde_json::Value>(
            requests[0]
                .body
                .as_ref()
                .expect("body present")
                .as_json_bytes()
                .expect("JSON request body")
        )
        .expect("valid JSON request body")["model"],
        "claude-sonnet-4-6"
    );

    assert_eq!(response.text(), "Done");
    assert_eq!(response.finish_reason, FinishReason::ToolCalls);
    assert_eq!(response.tool_calls().len(), 1);
    assert_eq!(response.tool_calls()[0].name, "memory_read");
    assert_eq!(response.structured_output, None);
    assert_eq!(response.usage.total_tokens, 202);
    assert_eq!(response.usage.input_tokens, 130);
    assert_eq!(response.usage.output_tokens, 32);
    assert_eq!(response.usage.cached_input_tokens, 40);
}

#[tokio::test]
async fn builds_anthropic_structured_output_requests_and_parses_response() {
    let (http_client, requests) = recording_http_client(JsonHttpResponse {
        headers: Default::default(),
        status: 200,
        body: json!({
            "stop_reason": "end_turn",
            "content": [
                {
                    "type": "text",
                    "text": "{\"summary\":\"Done\",\"done\":true}"
                }
            ],
            "usage": {
                "input_tokens": 12,
                "output_tokens": 6
            }
        }),
    });
    let model = AnthropicModel::new(http_client, CLAUDE_SONNET_4_6, "anthropic-key");

    let response = model
        .complete(&ModelRequest {
            system_prompt: "system".to_owned(),
            messages: vec![ConversationMessage::user("hello")],
            tools: Vec::new(),
            response_schema: Some(StructuredOutputSchema {
                name: "status".to_owned(),
                schema: json!({
                    "type": "object",
                    "properties": {
                        "summary": {"type": "string"},
                        "done": {"type": "boolean"}
                    },
                    "required": ["summary", "done"]
                }),
            }),
            controls: Default::default(),
        })
        .await
        .expect("Anthropic structured response should parse");

    let requests = requests
        .lock()
        .expect("requests lock should not be poisoned");
    assert!(
        serde_json::from_slice::<serde_json::Value>(
            requests[0]
                .body
                .as_ref()
                .expect("body present")
                .as_json_bytes()
                .expect("JSON request body")
        )
        .expect("valid JSON request body")["system"]
            .as_str()
            .expect("system prompt should be a string")
            .contains("return raw JSON only")
    );
    assert_eq!(
        response.structured_output,
        Some(json!({
            "summary": "Done",
            "done": true
        }))
    );
    assert_eq!(response.finish_reason, FinishReason::Stop);
}

#[tokio::test]
async fn maps_anthropic_stop_reasons() {
    let cases = [
        ("end_turn", FinishReason::Stop, false),
        ("stop_sequence", FinishReason::Stop, false),
        ("tool_use", FinishReason::ToolCalls, true),
        ("max_tokens", FinishReason::Truncated, false),
        (
            "model_context_window_exceeded",
            FinishReason::Truncated,
            false,
        ),
        ("refusal", FinishReason::Filtered, false),
        (
            "pause_turn",
            FinishReason::Other("pause_turn".to_owned()),
            false,
        ),
        (
            "custom_reason",
            FinishReason::Other("custom_reason".to_owned()),
            false,
        ),
    ];

    for (raw_reason, expected, include_tool) in cases {
        let (http_client, _) = recording_http_client(JsonHttpResponse {
            headers: Default::default(),
            status: 200,
            body: anthropic_response_body(raw_reason, include_tool),
        });
        let model = AnthropicModel::new(http_client, CLAUDE_SONNET_4_6, "anthropic-key");

        let response = model
            .complete(&simple_request())
            .await
            .expect("Anthropic response should parse");

        assert_eq!(response.finish_reason, expected);
    }
}

#[tokio::test]
async fn maps_missing_anthropic_stop_reason_to_other() {
    let (http_client, _) = recording_http_client(JsonHttpResponse {
        headers: Default::default(),
        status: 200,
        body: json!({
            "content": [
                { "type": "text", "text": "Done" }
            ],
            "usage": {
                "input_tokens": 1,
                "output_tokens": 1
            }
        }),
    });
    let model = AnthropicModel::new(http_client, CLAUDE_SONNET_4_6, "anthropic-key");

    let response = model
        .complete(&simple_request())
        .await
        .expect("Anthropic response should parse");

    assert_eq!(
        response.finish_reason,
        FinishReason::Other("missing".to_owned())
    );
}

fn recording_http_client(
    response: JsonHttpResponse<serde_json::Value>,
) -> (Arc<dyn JsonHttpClient>, RecordedRequests) {
    client_for_buffered_bodies(vec![response.body])
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

fn anthropic_response_body(raw_reason: &str, include_tool: bool) -> serde_json::Value {
    let content = if include_tool {
        json!([{
            "type": "tool_use",
            "id": "call_1",
            "name": "memory_read",
            "input": { "path": "root" }
        }])
    } else {
        json!([{ "type": "text", "text": "Done" }])
    };
    json!({
        "stop_reason": raw_reason,
        "content": content,
        "usage": {
            "input_tokens": 1,
            "output_tokens": 1
        }
    })
}
