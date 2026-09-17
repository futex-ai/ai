//! TypeSafe judgment usage normalization tests.

use std::collections::BTreeMap;

use ai_interface::{JudgmentQuestion, JudgmentRequest, ModelUsage};
use serde_json::{Value, json};

use super::parse_response;

#[test]
fn provider_usage_maps_and_sums_tokens() {
    let response = parse_response(
        "jev-latest",
        &request(),
        body_with_usage(Some(json!({"input_tokens": 312, "output_tokens": 48}))),
    )
    .expect("response should parse");

    assert_eq!(
        response.usage,
        ModelUsage {
            input_tokens: 312,
            output_tokens: 48,
            total_tokens: 360,
            ..ModelUsage::default()
        }
    );
}

#[test]
fn missing_usage_uses_the_shared_default() {
    let response = parse_response("jev-latest", &request(), body_with_usage(None))
        .expect("response should parse");

    assert_eq!(response.usage, ModelUsage::default());
}

#[test]
fn usage_total_saturates_and_other_buckets_stay_empty() {
    let response = parse_response(
        "jev-latest",
        &request(),
        body_with_usage(Some(json!({"input_tokens": u64::MAX, "output_tokens": 9}))),
    )
    .expect("response should parse");

    assert_eq!(response.usage.input_tokens, u64::MAX);
    assert_eq!(response.usage.output_tokens, 9);
    assert_eq!(response.usage.total_tokens, u64::MAX);
    assert_eq!(response.usage.cached_input_tokens, 0);
    assert_eq!(response.usage.reasoning_tokens, 0);
    assert_eq!(response.usage.estimated_cost_microusd, 0);
    assert!(response.usage.cost_lines.is_empty());
}

fn request() -> JudgmentRequest {
    JudgmentRequest {
        state: "ticket".into(),
        questions: BTreeMap::from([(
            "is_urgent".to_owned(),
            JudgmentQuestion::condition("Urgent?", None),
        )]),
    }
}

fn body_with_usage(usage: Option<Value>) -> Value {
    let mut body = json!({
        "model": "jev-1.13.0",
        "answers": {"is_urgent": {"type": "noul", "noul": 0.8}}
    });
    if let Some(usage) = usage {
        body["usage"] = usage;
    }
    body
}
