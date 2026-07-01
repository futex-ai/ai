//! Usage response parsing tests.

use serde_json::json;

use super::{ThinkingLevel, parse_response};

#[test]
fn parses_usage_token_details() {
    let response = parse_response(
        "gpt-5.5-thinking-medium",
        "gpt-5.5",
        ThinkingLevel::Medium,
        json!({
            "status": "completed",
            "output": [{
                "type": "message",
                "role": "assistant",
                "content": [{ "type": "output_text", "text": "Done" }]
            }],
            "usage": {
                "input_tokens": 120,
                "output_tokens": 32,
                "total_tokens": 152,
                "input_tokens_details": { "cached_tokens": 40 },
                "output_tokens_details": { "reasoning_tokens": 12 }
            }
        }),
        None,
    )
    .expect("usage details should parse");

    assert_eq!(response.usage.input_tokens, 80);
    assert_eq!(response.usage.output_tokens, 20);
    assert_eq!(response.usage.total_tokens, 152);
    assert_eq!(response.usage.cached_input_tokens, 40);
    assert_eq!(response.usage.reasoning_tokens, 12);
}

#[test]
fn missing_total_tokens_falls_back_to_normalized_usage_sum() {
    let response = parse_response(
        "gpt-5.5-thinking-medium",
        "gpt-5.5",
        ThinkingLevel::Medium,
        json!({
            "status": "completed",
            "output": [{
                "type": "message",
                "role": "assistant",
                "content": [{ "type": "output_text", "text": "Done" }]
            }],
            "usage": {
                "input_tokens": 120,
                "output_tokens": 32,
                "input_tokens_details": { "cached_tokens": 40 },
                "output_tokens_details": { "reasoning_tokens": 12 }
            }
        }),
        None,
    )
    .expect("usage details should parse");

    assert_eq!(response.usage.input_tokens, 80);
    assert_eq!(response.usage.output_tokens, 20);
    assert_eq!(response.usage.cached_input_tokens, 40);
    assert_eq!(response.usage.reasoning_tokens, 12);
    assert_eq!(response.usage.total_tokens, 152);
}
