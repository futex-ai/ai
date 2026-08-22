//! xAI synchronous streaming request and buffered-parity tests.

use std::time::Duration;

use ai_interface::{ConversationMessage, Model, ModelRequest, StructuredOutputSchema};
use ai_models_core::{
    ThinkingLevel, synthetic_tool_call_scope,
    test_support::{SseFixture, done_event, event, recording_streaming_client},
};
use serde_json::{Value, json};

use super::{XaiModel, response};

const MODEL_ID: &str = "grok-4.5";

#[tokio::test]
async fn streams_rich_deltas_with_usage_and_matches_buffered_parser() {
    let request = simple_request();
    let buffered = rich_buffered_body();
    let events = vec![
        event(json!({
            "choices": [{
                "index": 0,
                "delta": {
                    "content": "Checking ",
                    "tool_calls": [{
                        "index": 0,
                        "id": "call_1",
                        "function": {"name": "lookup", "arguments": "{\"id\":"}
                    }]
                },
                "finish_reason": null
            }],
            "usage": {"prompt_tokens": 20, "completion_tokens": 1}
        })),
        event(json!({
            "choices": [{
                "index": 0,
                "delta": {
                    "content": "now",
                    "tool_calls": [{"index": 0, "function": {"arguments": "7}"}}]
                },
                "finish_reason": "tool_calls"
            }]
        })),
        event(json!({"choices": [], "usage": buffered["usage"].clone()})),
        done_event(),
    ];
    let (http_client, requests) = recording_streaming_client(vec![SseFixture::Stream(events)]);
    let model = XaiModel::new(http_client, MODEL_ID, "xai-key");

    let streamed = model
        .complete(&request)
        .await
        .expect("xAI stream should parse");
    let parsed = response::parse_response(
        MODEL_ID,
        MODEL_ID,
        ThinkingLevel::Disabled,
        &synthetic_tool_call_scope(&request),
        buffered,
        None,
    )
    .expect("buffered xAI response should parse");

    assert_eq!(streamed, parsed);
    let requests = requests.lock().expect("request lock should be available");
    let request = &requests[0];
    let body = request
        .body
        .as_ref()
        .expect("request body should be present");
    assert_eq!(body["stream"], true);
    assert_eq!(body["stream_options"]["include_usage"], true);
    assert!(body.get("deferred").is_none());
    assert_eq!(request.timeout, Duration::from_secs(3_600));
    assert_eq!(request.idle_timeout, Some(Duration::from_secs(120)));
}

#[tokio::test]
async fn streamed_structured_output_matches_buffered_parser() {
    let schema = StructuredOutputSchema {
        name: "result".to_owned(),
        schema: json!({
            "type": "object",
            "properties": {"ok": {"type": "boolean"}},
            "required": ["ok"]
        }),
    };
    let body = json!({
        "choices": [{
            "message": {"content": "{\"ok\":true}"},
            "finish_reason": "stop"
        }],
        "usage": {"prompt_tokens": 4, "completion_tokens": 3, "total_tokens": 7}
    });
    let events = vec![
        event(json!({
            "choices": [{
                "index": 0,
                "delta": {"content": "{\"ok\":"},
                "finish_reason": null
            }]
        })),
        event(json!({
            "choices": [{
                "index": 0,
                "delta": {"content": "true}"},
                "finish_reason": "stop"
            }],
            "usage": body["usage"].clone()
        })),
        done_event(),
    ];
    let (http_client, _) = recording_streaming_client(vec![SseFixture::Stream(events)]);
    let model = XaiModel::new(http_client, MODEL_ID, "key");
    let mut request = simple_request();
    request.response_schema = Some(schema.clone());

    let streamed = model
        .complete(&request)
        .await
        .expect("structured stream should parse");
    let parsed = response::parse_response(
        MODEL_ID,
        MODEL_ID,
        ThinkingLevel::Disabled,
        &synthetic_tool_call_scope(&request),
        body,
        Some(&schema),
    )
    .expect("buffered structured response should parse");

    assert_eq!(streamed, parsed);
}

#[tokio::test]
async fn legacy_function_call_deltas_match_buffered_parser() {
    let request = simple_request();
    let body = json!({
        "choices": [{
            "message": {
                "content": "Checking",
                "function_call": {"name": "lookup", "arguments": "{\"id\":7}"}
            },
            "finish_reason": "function_call"
        }],
        "usage": {"prompt_tokens": 4, "completion_tokens": 3, "total_tokens": 7}
    });
    let events = vec![
        event(json!({
            "choices": [{
                "index": 0,
                "delta": {
                    "content": "Checking",
                    "function_call": {"name": "lookup", "arguments": "{\"id\":"}
                },
                "finish_reason": null
            }]
        })),
        event(json!({
            "choices": [{
                "index": 0,
                "delta": {"function_call": {"arguments": "7}"}},
                "finish_reason": "function_call"
            }],
            "usage": body["usage"].clone()
        })),
        done_event(),
    ];
    let (http_client, _) = recording_streaming_client(vec![SseFixture::Stream(events)]);
    let model = XaiModel::new(http_client, MODEL_ID, "key");

    let streamed = model
        .complete(&request)
        .await
        .expect("legacy function stream should parse");
    let parsed = response::parse_response(
        MODEL_ID,
        MODEL_ID,
        ThinkingLevel::Disabled,
        &synthetic_tool_call_scope(&request),
        body,
        None,
    )
    .expect("buffered legacy function response should parse");

    assert_eq!(streamed, parsed);
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

fn rich_buffered_body() -> Value {
    json!({
        "choices": [{
            "message": {
                "content": "Checking now",
                "tool_calls": [{
                    "id": "call_1",
                    "function": {"name": "lookup", "arguments": "{\"id\":7}"}
                }]
            },
            "finish_reason": "tool_calls"
        }],
        "usage": {
            "prompt_tokens": 20,
            "completion_tokens": 9,
            "total_tokens": 29,
            "prompt_tokens_details": {"cached_tokens": 5},
            "completion_tokens_details": {"reasoning_tokens": 4}
        }
    })
}
