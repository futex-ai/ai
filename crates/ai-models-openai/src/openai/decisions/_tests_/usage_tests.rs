//! OpenAI Decisions usage normalization tests.

use std::collections::BTreeMap;

use ai_interface::{
    JudgmentQuestion, JudgmentRequest, JudgmentResponse, JudgmentResult, ModelUsage,
};
use serde_json::{Value, json};

use super::super::response::parse_response;

#[test]
fn provider_usage_separates_cached_and_reasoning_buckets() {
    let response = parse_value(body_with_usage(Some(json!({
        "input_tokens": 312,
        "input_tokens_details": {"cached_tokens": 100, "cache_write_tokens": 7},
        "output_tokens": 48,
        "output_tokens_details": {"reasoning_tokens": 40},
        "total_tokens": 360
    }))))
    .expect("response should parse");

    assert_eq!(
        response.usage,
        ModelUsage {
            input_tokens: 212,
            output_tokens: 8,
            cached_input_tokens: 100,
            reasoning_tokens: 40,
            total_tokens: 360,
            ..ModelUsage::default()
        }
    );
}

#[test]
fn missing_usage_uses_the_shared_default() {
    let response = parse_value(body_with_usage(None)).expect("response should parse");

    assert_eq!(response.usage, ModelUsage::default());
}

#[test]
fn null_usage_object_uses_the_shared_default() {
    let response =
        parse_value(body_with_usage(Some(Value::Null))).expect("null usage should not fail");

    assert_eq!(response.usage, ModelUsage::default());
}

#[test]
fn null_or_absent_counters_map_to_zero_and_total_is_reconstructed() {
    for usage in [
        json!({"input_tokens": null, "output_tokens": null, "total_tokens": null}),
        json!({"input_tokens": 12, "input_tokens_details": null}),
        json!({"input_tokens": 12, "input_tokens_details": {"cached_tokens": null}}),
        json!({"output_tokens": 3}),
        json!({}),
    ] {
        let response = parse_value(body_with_usage(Some(usage.clone())))
            .expect("nullable usage counters should not fail the response");
        let input_tokens = usage["input_tokens"].as_u64().unwrap_or_default();
        let output_tokens = usage["output_tokens"].as_u64().unwrap_or_default();

        assert_eq!(
            response.usage,
            ModelUsage {
                input_tokens,
                output_tokens,
                total_tokens: input_tokens + output_tokens,
                ..ModelUsage::default()
            },
            "usage fixture {usage}"
        );
    }
}

#[test]
fn subtraction_and_total_saturate() {
    let response = parse_value(body_with_usage(Some(json!({
        "input_tokens": 5,
        "input_tokens_details": {"cached_tokens": 9},
        "output_tokens": u64::MAX,
        "output_tokens_details": {"reasoning_tokens": 1}
    }))))
    .expect("response should parse");

    assert_eq!(response.usage.input_tokens, 0);
    assert_eq!(response.usage.cached_input_tokens, 9);
    assert_eq!(response.usage.output_tokens, u64::MAX - 1);
    assert_eq!(response.usage.reasoning_tokens, 1);
    assert_eq!(response.usage.total_tokens, u64::MAX);
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
        "model": "gpt-6-luna",
        "answers": [{"type": "predicate", "name": "is_urgent", "probability": 0.8}]
    });
    if let Some(usage) = usage {
        body["usage"] = usage;
    }
    body
}

fn parse_value(body: Value) -> JudgmentResult<JudgmentResponse> {
    let body = serde_json::to_vec(&body).expect("response fixture should serialize");
    parse_response("gpt-6-luna", &request(), &body)
}
