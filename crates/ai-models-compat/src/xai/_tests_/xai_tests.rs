//! Tests for xAI request mapping and response parsing.

use ai_interface::{
    ConversationMessage, ConversationRole, FinishReason, Model, ModelRequest,
    StructuredOutputSchema, ToolDefinition,
};
use ai_models_core::{ThinkingLevel, synthetic_tool_call_scope};
use json_http::JsonHttpResponse;
use serde_json::json;

use super::{XaiModel, response, test_support::recording_http_client};

#[tokio::test]
async fn builds_xai_tool_requests_and_parses_response() {
    let (http_client, requests) = recording_http_client(JsonHttpResponse {
        status: 200,
        body: json!({
            "choices": [{
                "finish_reason": "tool_calls",
                "message": {
                    "content": "Done",
                    "tool_calls": [{
                        "id": "call_1",
                        "function": {
                            "name": "memory_read",
                            "arguments": "{\"path\":\"root\"}"
                        }
                    }]
                }
            }],
            "usage": {
                "prompt_tokens": 120,
                "completion_tokens": 32,
                "total_tokens": 152
            }
        }),
    });
    let model = XaiModel::new(http_client, "grok-4", "xai-key");

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
            controls: Default::default(),
        })
        .await
        .expect("xAI response should parse");

    let requests = requests
        .lock()
        .expect("requests lock should not be poisoned");
    assert_eq!(requests[0].url, "https://api.x.ai/v1/chat/completions");
    assert_eq!(
        requests[0].headers.get("Authorization"),
        Some(&"Bearer xai-key".to_owned())
    );
    assert_eq!(
        requests[0].body.as_ref().expect("body present")["model"],
        "grok-4"
    );
    assert_eq!(
        requests[0].body.as_ref().expect("body present")["tools"][0]["function"]["name"],
        "memory_read"
    );

    assert_eq!(response.assistant_message, "Done");
    assert_eq!(response.finish_reason, FinishReason::ToolCalls);
    assert_eq!(response.tool_calls.len(), 1);
    assert_eq!(response.tool_calls[0].name, "memory_read");
    assert_eq!(response.structured_output, None);
    assert_eq!(response.usage.total_tokens, 152);
    assert_eq!(response.usage.cached_input_tokens, 0);
    assert_eq!(response.usage.reasoning_tokens, 0);
}

#[tokio::test]
async fn builds_xai_structured_output_requests_and_parses_response() {
    let (http_client, requests) = recording_http_client(JsonHttpResponse {
        status: 200,
        body: json!({
            "choices": [{
                "finish_reason": "stop",
                "message": {
                    "content": "{\"summary\":\"Done\",\"done\":true}",
                    "tool_calls": []
                }
            }],
            "usage": {
                "prompt_tokens": 12,
                "completion_tokens": 6,
                "total_tokens": 18
            }
        }),
    });
    let model = XaiModel::new(http_client, "grok-4", "xai-key");

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
        .expect("xAI structured response should parse");

    let requests = requests
        .lock()
        .expect("requests lock should not be poisoned");
    assert_eq!(
        requests[0].body.as_ref().expect("body present")["response_format"]["type"],
        "json_schema"
    );
    assert_eq!(
        requests[0].body.as_ref().expect("body present")["response_format"]["json_schema"]["strict"],
        false
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
async fn maps_xai_finish_reasons() {
    let cases = [
        ("stop", FinishReason::Stop, false),
        ("tool_calls", FinishReason::ToolCalls, true),
        ("function_call", FinishReason::ToolCalls, false),
        ("length", FinishReason::Truncated, false),
        ("content_filter", FinishReason::Filtered, false),
        (
            "custom_reason",
            FinishReason::Other("custom_reason".to_owned()),
            false,
        ),
    ];

    for (raw_reason, expected, include_tool) in cases {
        let (http_client, _) = recording_http_client(JsonHttpResponse {
            status: 200,
            body: xai_response_body(raw_reason, include_tool),
        });
        let model = XaiModel::new(http_client, "grok-4", "xai-key");

        let response = model
            .complete(&simple_request())
            .await
            .expect("xAI response should parse");

        assert_eq!(response.finish_reason, expected);
    }
}

#[test]
fn maps_missing_xai_finish_reason_to_other() {
    let request = simple_request();
    let response = response::parse_response(
        "grok-4",
        "grok-4",
        ThinkingLevel::Disabled,
        &synthetic_tool_call_scope(&request),
        json!({
            "choices": [{
                "message": {
                    "content": "Done",
                    "tool_calls": []
                }
            }]
        }),
        None,
    )
    .expect("xAI response should parse");

    assert_eq!(
        response.finish_reason,
        FinishReason::Other("missing".to_owned())
    );
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

fn xai_response_body(raw_reason: &str, include_tool: bool) -> serde_json::Value {
    let tool_calls = if include_tool {
        json!([{
            "id": "call_1",
            "function": {
                "name": "memory_read",
                "arguments": "{\"path\":\"root\"}"
            }
        }])
    } else {
        json!([])
    };
    json!({
        "choices": [{
            "finish_reason": raw_reason,
            "message": {
                "content": "Done",
                "tool_calls": tool_calls
            }
        }]
    })
}
