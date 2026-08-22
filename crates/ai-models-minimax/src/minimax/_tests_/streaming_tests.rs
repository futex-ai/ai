//! MiniMax streaming request and buffered-parity tests.

use std::time::Duration;

use ai_interface::{Model, StructuredOutputSchema};
use ai_models_core::{
    ThinkingLevel,
    test_support::{SseFixture, done_event, event, recording_streaming_client},
};
use serde_json::{Value, json};

use crate::{MINIMAX_M3, MiniMaxModel};

use super::{response, support::simple_request};

#[tokio::test]
async fn normalizes_cumulative_content_and_preserves_reasoning_details() {
    let buffered = rich_buffered_body();
    let events = vec![
        event(json!({
            "choices": [{
                "index": 0,
                "delta": {
                    "content": "Checking ",
                    "reasoning_details": [{
                        "type": "reasoning.text",
                        "id": "reasoning-1",
                        "format": "MiniMax-response-v1",
                        "index": 0,
                        "text": "private "
                    }],
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
                    "content": "Checking now",
                    "reasoning_details": [{
                        "type": "reasoning.text",
                        "id": "reasoning-1",
                        "format": "MiniMax-response-v1",
                        "index": 0,
                        "text": "private reasoning"
                    }],
                    "tool_calls": [{"index": 0, "function": {"arguments": "7}"}}]
                },
                "finish_reason": "tool_calls"
            }]
        })),
        event(json!({"choices": [], "usage": buffered["usage"].clone()})),
        done_event(),
    ];
    let (http_client, requests) = recording_streaming_client(vec![SseFixture::Stream(events)]);
    let model = MiniMaxModel::new(http_client, MINIMAX_M3, "minimax-key");

    let streamed = model
        .complete(&simple_request())
        .await
        .expect("MiniMax stream should parse");
    let parsed = response::parse_response(
        MINIMAX_M3,
        MINIMAX_M3,
        ThinkingLevel::Medium,
        buffered,
        None,
    )
    .expect("buffered MiniMax response should parse");

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
async fn retains_latest_revised_reasoning_snapshot_for_tool_call() {
    let buffered = json!({
        "choices": [{
            "message": {
                "reasoning_details": [{
                    "type": "reasoning.text",
                    "id": "reasoning-1",
                    "format": "MiniMax-response-v1",
                    "index": 0,
                    "text": "Call the required tool."
                }],
                "tool_calls": [{
                    "id": "call_1",
                    "function": {
                        "name": "live_probe",
                        "arguments": "{\"token\":\"MINIMAX_REQUIRED_OK\"}"
                    }
                }]
            },
            "finish_reason": "tool_calls"
        }],
        "usage": {"prompt_tokens": 20, "completion_tokens": 9, "total_tokens": 29}
    });
    let events = vec![
        event(json!({
            "choices": [{
                "index": 0,
                "delta": {
                    "reasoning_details": [{
                        "type": "reasoning.text",
                        "id": "reasoning-1",
                        "format": "MiniMax-response-v1",
                        "index": 0,
                        "text": "Answer without a tool."
                    }]
                },
                "finish_reason": null
            }]
        })),
        event(json!({
            "choices": [{
                "index": 0,
                "delta": {
                    "reasoning_details": buffered["choices"][0]["message"]
                        ["reasoning_details"].clone(),
                    "tool_calls": [{
                        "index": 0,
                        "id": "call_1",
                        "function": {
                            "name": "live_probe",
                            "arguments": "{\"token\":"
                        }
                    }]
                },
                "finish_reason": null
            }]
        })),
        event(json!({
            "choices": [{
                "index": 0,
                "delta": {
                    "tool_calls": [{
                        "index": 0,
                        "function": {"arguments": "\"MINIMAX_REQUIRED_OK\"}"}
                    }]
                },
                "finish_reason": "tool_calls"
            }]
        })),
        event(json!({"choices": [], "usage": buffered["usage"].clone()})),
        done_event(),
    ];
    let (http_client, _) = recording_streaming_client(vec![SseFixture::Stream(events)]);

    let streamed = MiniMaxModel::new(http_client, MINIMAX_M3, "key")
        .complete(&simple_request())
        .await
        .expect("revised reasoning snapshot should parse");
    let parsed = response::parse_response(
        MINIMAX_M3,
        MINIMAX_M3,
        ThinkingLevel::Medium,
        buffered,
        None,
    )
    .expect("buffered MiniMax response should parse");

    assert_eq!(streamed, parsed);
}

#[tokio::test]
async fn cumulative_structured_output_matches_buffered_parser() {
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
                "delta": {"content": "{\"ok\":true}"},
                "finish_reason": "stop"
            }],
            "usage": body["usage"].clone()
        })),
        done_event(),
    ];
    let (http_client, _) = recording_streaming_client(vec![SseFixture::Stream(events)]);
    let model = MiniMaxModel::new(http_client, MINIMAX_M3, "key");
    let mut request = simple_request();
    request.response_schema = Some(schema.clone());

    let streamed = model
        .complete(&request)
        .await
        .expect("structured stream should parse");
    let parsed = response::parse_response(
        MINIMAX_M3,
        MINIMAX_M3,
        ThinkingLevel::Medium,
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
                "reasoning_details": [{
                    "type": "reasoning.text",
                    "id": "reasoning-1",
                    "format": "MiniMax-response-v1",
                    "index": 0,
                    "text": "private reasoning"
                }],
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
