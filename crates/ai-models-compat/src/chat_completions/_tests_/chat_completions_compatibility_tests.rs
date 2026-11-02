//! Compatibility tests for provider-specific Chat Completions stream shapes.

use serde_json::json;

use crate::{ChatCompletionsAccumulator, ChatCompletionsStreamError};

#[test]
fn rejects_standard_provider_error_payloads_before_chunk_accumulation() {
    let error = ChatCompletionsAccumulator::new()
        .push_data(r#"{"error":{"code":"bad_request","message":"invalid"}}"#)
        .expect_err("provider error payload should not be treated as an empty chunk");

    assert!(matches!(
        error,
        ChatCompletionsStreamError::ProviderEvent { .. }
    ));
}

#[test]
fn preserves_direct_cached_tokens_and_legacy_function_call_deltas() {
    let mut accumulator = ChatCompletionsAccumulator::new();
    for chunk in [
        json!({
            "choices": [{
                "index": 0,
                "delta": {
                    "content": "Working",
                    "function_call": {"name": "lookup", "arguments": "{\"id\":"}
                },
                "finish_reason": null
            }]
        }),
        json!({
            "choices": [{
                "index": 0,
                "delta": {"function_call": {"arguments": "7}"}},
                "finish_reason": "function_call"
            }],
            "usage": {
                "prompt_tokens": 12,
                "completion_tokens": 4,
                "total_tokens": 16,
                "cached_tokens": 5
            }
        }),
    ] {
        accumulator
            .push_data(&chunk.to_string())
            .expect("valid compatibility chunk should accumulate");
    }
    accumulator
        .push_data("[DONE]")
        .expect("terminal sentinel should succeed");

    let value = serde_json::to_value(
        accumulator
            .finish()
            .expect("complete compatibility stream should finish"),
    )
    .expect("accumulated response should serialize");

    assert_eq!(value["usage"]["cached_tokens"], 5);
    assert_eq!(
        value["choices"][0]["message"]["function_call"],
        json!({"name": "lookup", "arguments": "{\"id\":7}"})
    );
}
