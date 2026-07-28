//! Kimi usage normalization tests.

use ai_models_core::ThinkingLevel;
use serde_json::{Value, json};

use crate::KIMI_K3;

use super::response::parse_response;

#[test]
fn separates_cached_input_and_keeps_completion_as_output() {
    let response = parse(usage_body(json!({
        "prompt_tokens": 120,
        "completion_tokens": 50,
        "total_tokens": 170,
        "prompt_tokens_details": {"cached_tokens": 80},
        "completion_tokens_details": {"reasoning_tokens": 20}
    })));

    assert_eq!(response.usage.input_tokens, 40);
    assert_eq!(response.usage.cached_input_tokens, 80);
    assert_eq!(response.usage.output_tokens, 50);
    assert_eq!(response.usage.reasoning_tokens, 0);
    assert_eq!(response.usage.total_tokens, 170);
}

#[test]
fn missing_usage_maps_to_zeroes() {
    let response = parse(json!({
        "choices": [{
            "finish_reason": "stop",
            "message": {"content": "Done"}
        }]
    }));

    assert_eq!(response.usage, ai_interface::ModelUsage::default());
}

#[test]
fn missing_total_uses_normalized_saturating_sum() {
    let ordinary = parse(usage_body(json!({
        "prompt_tokens": 120,
        "completion_tokens": 32,
        "prompt_tokens_details": {"cached_tokens": 80}
    })));
    assert_eq!(ordinary.usage.total_tokens, 152);

    let inconsistent = parse(usage_body(json!({
        "prompt_tokens": 2,
        "completion_tokens": 3,
        "prompt_tokens_details": {"cached_tokens": 5}
    })));
    assert_eq!(inconsistent.usage.input_tokens, 0);
    assert_eq!(inconsistent.usage.total_tokens, 8);
}

#[test]
fn usage_remains_unpriced_in_provider_crate() {
    let response = parse(usage_body(json!({
        "prompt_tokens": 2,
        "completion_tokens": 3,
        "total_tokens": 99
    })));

    assert_eq!(response.usage.total_tokens, 99);
    assert_eq!(response.usage.estimated_cost_microusd, 0);
    assert!(response.usage.cost_lines.is_empty());
}

fn parse(body: Value) -> ai_interface::ModelResponse {
    parse_response(KIMI_K3, KIMI_K3, ThinkingLevel::Max, body, None)
        .expect("usage response should parse")
}

fn usage_body(usage: Value) -> Value {
    json!({
        "choices": [{
            "finish_reason": "stop",
            "message": {"content": "Done"}
        }],
        "usage": usage
    })
}
