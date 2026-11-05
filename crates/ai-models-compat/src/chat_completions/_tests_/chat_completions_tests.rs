//! Tests for OpenAI-compatible chat-completions stream accumulation.

use serde_json::json;

use crate::{ChatCompletionsAccumulator, ChatCompletionsStreamError, ChatCompletionsStreamStatus};

#[test]
fn accumulates_content_reasoning_finish_and_final_usage() {
    let mut accumulator = ChatCompletionsAccumulator::new();
    push(
        &mut accumulator,
        &json!({
            "choices": [{
                "index": 0,
                "delta": {"content": "Hello", "reasoning_content": "Think"},
                "finish_reason": null
            }]
        }),
    );
    push(
        &mut accumulator,
        &json!({
            "choices": [{
                "index": 0,
                "delta": {"content": " world", "reasoning_content": "ing"},
                "finish_reason": "stop"
            }]
        }),
    );
    push(
        &mut accumulator,
        &json!({
            "choices": [],
            "usage": {
                "prompt_tokens": 10,
                "completion_tokens": 7,
                "total_tokens": 17,
                "prompt_tokens_details": {"cached_tokens": 4},
                "completion_tokens_details": {"reasoning_tokens": 3},
                "prompt_cache_hit_tokens": 2,
                "prompt_cache_miss_tokens": 8
            }
        }),
    );

    assert_eq!(
        accumulator
            .push_data("[DONE]")
            .expect("done sentinel should succeed"),
        ChatCompletionsStreamStatus::Done
    );
    let response = accumulator.finish().expect("complete stream should finish");
    let choice = &response.choices[0];

    assert_eq!(choice.message.content.as_deref(), Some("Hello world"));
    assert_eq!(
        choice.message.reasoning_content.as_deref(),
        Some("Thinking")
    );
    assert_eq!(choice.finish_reason, "stop");
    assert_eq!(response.usage.prompt_tokens, 10);
    assert_eq!(response.usage.prompt_tokens_details.cached_tokens, 4);
    assert_eq!(response.usage.completion_tokens_details.reasoning_tokens, 3);
    assert_eq!(response.usage.prompt_cache_hit_tokens, 2);
    assert_eq!(response.usage.prompt_cache_miss_tokens, Some(8));
}

#[test]
fn joins_interleaved_indexed_tool_call_fragments() {
    let mut accumulator = ChatCompletionsAccumulator::new();
    for chunk in [
        json!({
            "choices": [{
                "index": 0,
                "delta": {"tool_calls": [{
                    "index": 1,
                    "id": "call_b",
                    "function": {"name": "write", "arguments": "{\"v\":"}
                }]},
                "finish_reason": null
            }]
        }),
        json!({
            "choices": [{
                "index": 0,
                "delta": {"tool_calls": [{
                    "index": 0,
                    "id": "call_a",
                    "function": {"name": "read", "arguments": "{\"k\":"}
                }, {
                    "index": 1,
                    "function": {"arguments": "2}"}
                }]},
                "finish_reason": null
            }]
        }),
        json!({
            "choices": [{
                "index": 0,
                "delta": {"tool_calls": [{
                    "index": 0,
                    "function": {"arguments": "1}"}
                }]},
                "finish_reason": "tool_calls"
            }],
            "usage": {"prompt_tokens": 5, "completion_tokens": 4}
        }),
    ] {
        push(&mut accumulator, &chunk);
    }
    accumulator
        .push_data("[DONE]")
        .expect("done sentinel should succeed");

    let response = accumulator.finish().expect("complete stream should finish");
    let calls = &response.choices[0].message.tool_calls;

    assert_eq!(calls.len(), 2);
    assert_eq!(calls[0].id, "call_a");
    assert_eq!(calls[0].function.name, "read");
    assert_eq!(calls[0].function.arguments, "{\"k\":1}");
    assert_eq!(calls[1].id, "call_b");
    assert_eq!(calls[1].function.arguments, "{\"v\":2}");
}

#[test]
fn serializes_to_the_buffered_chat_completions_shape() {
    let mut accumulator = ChatCompletionsAccumulator::new();
    push(
        &mut accumulator,
        &json!({
            "choices": [{
                "index": 0,
                "delta": {"content": "done"},
                "finish_reason": "stop"
            }],
            "usage": {"prompt_tokens": 1, "completion_tokens": 1, "total_tokens": 2}
        }),
    );
    accumulator
        .push_data("[DONE]")
        .expect("done sentinel should succeed");
    let response = accumulator.finish().expect("complete stream should finish");

    assert_eq!(
        serde_json::to_value(response).expect("response should serialize"),
        json!({
            "choices": [{
                "index": 0,
                "message": {
                    "content": "done",
                    "reasoning_content": null,
                    "tool_calls": []
                },
                "finish_reason": "stop"
            }],
            "usage": {
                "prompt_tokens": 1,
                "completion_tokens": 1,
                "total_tokens": 2,
                "prompt_cache_hit_tokens": 0,
                "prompt_cache_miss_tokens": null,
                "prompt_tokens_details": {"cached_tokens": 0},
                "completion_tokens_details": {"reasoning_tokens": 0}
            }
        })
    );
}

#[test]
fn rejects_malformed_or_incomplete_streams() {
    let malformed = ChatCompletionsAccumulator::new()
        .push_data("not json")
        .expect_err("malformed chunk should fail");
    assert!(matches!(
        malformed,
        ChatCompletionsStreamError::DeserializeChunk { .. }
    ));

    let mut missing_done = ChatCompletionsAccumulator::new();
    push(
        &mut missing_done,
        &json!({
            "choices": [{"index": 0, "delta": {"content": "x"}, "finish_reason": "stop"}],
            "usage": {"prompt_tokens": 1, "completion_tokens": 1}
        }),
    );
    assert!(matches!(
        missing_done.finish(),
        Err(ChatCompletionsStreamError::MissingDone)
    ));

    let mut missing_usage = ChatCompletionsAccumulator::new();
    push(
        &mut missing_usage,
        &json!({
            "choices": [{"index": 0, "delta": {"content": "x"}, "finish_reason": "stop"}]
        }),
    );
    missing_usage
        .push_data("[DONE]")
        .expect("done sentinel should succeed");
    assert!(matches!(
        missing_usage.finish(),
        Err(ChatCompletionsStreamError::MissingUsage)
    ));
}

#[test]
fn rejects_events_after_done_and_incomplete_tool_calls() {
    let mut after_done = ChatCompletionsAccumulator::new();
    after_done
        .push_data("[DONE]")
        .expect("done sentinel should succeed");
    assert!(matches!(
        after_done.push_data("[DONE]"),
        Err(ChatCompletionsStreamError::EventAfterDone)
    ));

    let mut incomplete = ChatCompletionsAccumulator::new();
    push(
        &mut incomplete,
        &json!({
            "choices": [{
                "index": 0,
                "delta": {"tool_calls": [{
                    "index": 0,
                    "id": "call_1",
                    "function": {"arguments": "{}"}
                }]},
                "finish_reason": "tool_calls"
            }],
            "usage": {"prompt_tokens": 1, "completion_tokens": 1}
        }),
    );
    incomplete
        .push_data("[DONE]")
        .expect("done sentinel should succeed");
    assert!(matches!(
        incomplete.finish(),
        Err(ChatCompletionsStreamError::MissingToolFunctionName {
            choice_index: 0,
            tool_index: 0
        })
    ));
}

fn push(accumulator: &mut ChatCompletionsAccumulator, value: &serde_json::Value) {
    assert_eq!(
        accumulator
            .push_data(&value.to_string())
            .expect("valid chunk should accumulate"),
        ChatCompletionsStreamStatus::Chunk
    );
}
