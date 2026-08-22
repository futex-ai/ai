//! Qwen streaming request and buffered-parity tests.

use std::time::Duration;

use ai_interface::{Model, StructuredOutputSchema};
use ai_models_core::{
    ThinkingLevel,
    test_support::{SseFixture, done_event, event, recording_streaming_client},
};
use serde_json::{Value, json};

use crate::{QWEN_3_7_PLUS, QwenModel};

use super::{response, test_support::simple_request};

#[tokio::test]
async fn streams_rich_deltas_with_usage_and_matches_buffered_parser() {
    let buffered = rich_buffered_body();
    let events = vec![
        event(json!({
            "choices": [{
                "index": 0,
                "delta": {
                    "content": "Checking ",
                    "reasoning_content": "private ",
                    "tool_calls": [{
                        "index": 0,
                        "id": "call_1",
                        "function": {"name": "lookup", "arguments": "{\"id\":"}
                    }]
                },
                "finish_reason": null
            }]
        })),
        event(json!({
            "choices": [{
                "index": 0,
                "delta": {
                    "content": "now",
                    "reasoning_content": "reasoning",
                    "tool_calls": [{"index": 0, "function": {"arguments": "7}"}}]
                },
                "finish_reason": "tool_calls"
            }]
        })),
        event(json!({"choices": [], "usage": buffered["usage"].clone()})),
        done_event(),
    ];
    let (http_client, requests) = recording_streaming_client(vec![SseFixture::Stream(events)]);
    let model = QwenModel::new(http_client, "qwen-key");

    let streamed = model
        .complete(&simple_request())
        .await
        .expect("Qwen stream should parse");
    let parsed = response::parse_response(
        QWEN_3_7_PLUS,
        QWEN_3_7_PLUS,
        ThinkingLevel::High,
        buffered,
        None,
    )
    .expect("buffered Qwen response should parse");

    assert_eq!(streamed, parsed);
    let requests = requests.lock().expect("request lock should be available");
    let request = &requests[0];
    let body = request
        .body
        .as_ref()
        .expect("request body should be present");
    assert_eq!(body["stream"], true);
    assert_eq!(body["stream_options"]["include_usage"], true);
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
            "message": {"content": "{\"ok\":true}", "reasoning_content": "checked"},
            "finish_reason": "stop"
        }],
        "usage": {"prompt_tokens": 4, "completion_tokens": 3, "total_tokens": 7}
    });
    let events = vec![
        event(json!({
            "choices": [{
                "index": 0,
                "delta": {"content": "{\"ok\":", "reasoning_content": "checked"},
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
    let model = QwenModel::new(http_client, "key");
    let mut request = simple_request();
    request.response_schema = Some(schema.clone());

    let streamed = model
        .complete(&request)
        .await
        .expect("structured stream should parse");
    let parsed = response::parse_response(
        QWEN_3_7_PLUS,
        QWEN_3_7_PLUS,
        ThinkingLevel::High,
        body,
        Some(&schema),
    )
    .expect("buffered structured response should parse");

    assert_eq!(streamed, parsed);
}

fn rich_buffered_body() -> Value {
    json!({
        "choices": [{
            "message": {
                "content": "Checking now",
                "reasoning_content": "private reasoning",
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
            "prompt_tokens_details": {"cached_tokens": 5}
        }
    })
}
