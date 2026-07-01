//! Tests for Anthropic request mapping and response parsing.

use std::{
    collections::VecDeque,
    sync::{Arc, Mutex},
};

use ai_interface::{
    ConversationMessage, ConversationRole, FinishReason, Model, ModelRequest,
    StructuredOutputSchema, ToolDefinition,
};
use json_http::{
    JsonHttpClient, JsonHttpRequest, JsonHttpResponse, JsonHttpTransportMock,
    TransportBackedJsonHttpClient,
};
use serde_json::json;
use unimock::{MockFn, Unimock, matching};

use super::AnthropicModel;

type RecordedRequests = Arc<Mutex<Vec<JsonHttpRequest>>>;

#[tokio::test]
async fn builds_anthropic_tool_requests_and_parses_response() {
    let (http_client, requests) = recording_http_client(JsonHttpResponse {
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
    let model = AnthropicModel::new(http_client, "claude-sonnet-4-6", "anthropic-key");

    let response = model
        .complete(&ModelRequest {
            system_prompt: "system".to_owned(),
            messages: vec![ConversationMessage {
                role: ConversationRole::User,
                content: "hello".to_owned(),
                content_parts: Vec::new(),
                name: None,
                tool_call_id: None,
                tool_calls: Vec::new(),
                provider_context: Vec::new(),
            }],
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
        requests[0].body.as_ref().expect("body present")["model"],
        "claude-sonnet-4-6"
    );

    assert_eq!(response.assistant_message, "Done");
    assert_eq!(response.finish_reason, FinishReason::ToolCalls);
    assert_eq!(response.tool_calls.len(), 1);
    assert_eq!(response.tool_calls[0].name, "memory_read");
    assert_eq!(response.structured_output, None);
    assert_eq!(response.usage.total_tokens, 202);
    assert_eq!(response.usage.input_tokens, 130);
    assert_eq!(response.usage.output_tokens, 32);
    assert_eq!(response.usage.cached_input_tokens, 40);
}

#[tokio::test]
async fn builds_anthropic_structured_output_requests_and_parses_response() {
    let (http_client, requests) = recording_http_client(JsonHttpResponse {
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
    let model = AnthropicModel::new(http_client, "claude-sonnet-4-6", "anthropic-key");

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
        })
        .await
        .expect("Anthropic structured response should parse");

    let requests = requests
        .lock()
        .expect("requests lock should not be poisoned");
    assert!(
        requests[0].body.as_ref().expect("body present")["system"]
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
            status: 200,
            body: anthropic_response_body(raw_reason, include_tool),
        });
        let model = AnthropicModel::new(http_client, "claude-sonnet-4-6", "anthropic-key");

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
    let model = AnthropicModel::new(http_client, "claude-sonnet-4-6", "anthropic-key");

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
    let requests = Arc::new(Mutex::new(Vec::new()));
    let responses = Arc::new(Mutex::new(VecDeque::from([response])));
    let transport = Arc::new(Unimock::new(
        JsonHttpTransportMock::execute
            .each_call(matching!(_))
            .answers_arc({
                let requests = requests.clone();
                let responses = responses.clone();
                Arc::new(move |_, request: &JsonHttpRequest| {
                    requests
                        .lock()
                        .expect("requests lock should not be poisoned")
                        .push(request.clone());
                    Ok(responses
                        .lock()
                        .expect("responses lock should not be poisoned")
                        .pop_front()
                        .expect("unexpected transport call"))
                })
            }),
    ));

    (
        Arc::new(TransportBackedJsonHttpClient::new(transport)),
        requests,
    )
}

fn simple_request() -> ModelRequest {
    ModelRequest {
        system_prompt: "system".to_owned(),
        messages: vec![ConversationMessage::user("hello")],
        tools: Vec::new(),
        response_schema: None,
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
