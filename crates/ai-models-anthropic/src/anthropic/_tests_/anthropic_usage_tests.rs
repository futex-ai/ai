//! Anthropic usage normalization tests.

use ai_models_core::ThinkingLevel;
use serde_json::json;

use super::response::parse_response;

#[test]
fn normalizes_disjoint_anthropic_input_usage_buckets() {
    let response = parse_response(
        "claude-sonnet-4-6",
        "claude-sonnet-4-6",
        ThinkingLevel::Disabled,
        json!({
            "stop_reason": "end_turn",
            "content": [{ "type": "text", "text": "Done" }],
            "usage": {
                "input_tokens": 120,
                "cache_creation_input_tokens": 10,
                "cache_read_input_tokens": 40,
                "output_tokens": 32
            }
        }),
        None,
    )
    .expect("Anthropic response should parse");

    assert_eq!(response.usage.input_tokens, 120);
    assert_eq!(response.usage.output_tokens, 32);
    assert_eq!(response.usage.cached_input_tokens, 40);
    assert_eq!(response.usage.cache_write_input_tokens, 10);
    assert_eq!(response.usage.reasoning_tokens, 0);
    assert_eq!(response.usage.total_tokens, 202);
}

#[test]
fn defaults_missing_anthropic_usage_fields_to_zero() {
    let response = parse_response(
        "claude-sonnet-4-6",
        "claude-sonnet-4-6",
        ThinkingLevel::Disabled,
        json!({
            "stop_reason": "end_turn",
            "content": [{ "type": "text", "text": "Done" }],
            "usage": {}
        }),
        None,
    )
    .expect("Anthropic response should parse");

    assert_eq!(response.usage.input_tokens, 0);
    assert_eq!(response.usage.output_tokens, 0);
    assert_eq!(response.usage.cached_input_tokens, 0);
    assert_eq!(response.usage.cache_write_input_tokens, 0);
    assert_eq!(response.usage.reasoning_tokens, 0);
    assert_eq!(response.usage.total_tokens, 0);
}
