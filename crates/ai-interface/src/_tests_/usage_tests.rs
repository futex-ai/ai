//! Tests for normalized model usage serialization.

use serde_json::json;

use crate::{ModelUsage, ModelUsageUnitKind};

#[test]
fn cache_write_input_tokens_round_trip() {
    let usage = ModelUsage {
        input_tokens: 10,
        output_tokens: 20,
        cached_input_tokens: 30,
        cache_write_input_tokens: 40,
        reasoning_tokens: 50,
        total_tokens: 150,
        estimated_cost_microusd: 60,
        cost_lines: Vec::new(),
    };

    let serialized = serde_json::to_value(&usage).expect("usage should serialize");
    assert_eq!(serialized["cache_write_input_tokens"], 40);

    let round_tripped: ModelUsage =
        serde_json::from_value(serialized).expect("usage should deserialize");
    assert_eq!(round_tripped, usage);
}

#[test]
fn cache_write_input_tokens_default_to_zero_when_absent() {
    let usage: ModelUsage = serde_json::from_value(json!({
        "input_tokens": 10,
        "output_tokens": 20,
        "cached_input_tokens": 30,
        "reasoning_tokens": 40,
        "total_tokens": 100,
        "estimated_cost_microusd": 50
    }))
    .expect("stored usage should deserialize");

    assert_eq!(usage.cache_write_input_tokens, 0);
}

#[test]
fn cache_write_input_token_kind_has_stable_name() {
    assert_eq!(
        ModelUsageUnitKind::CacheWriteInputToken.as_str(),
        "cache_write_input_token"
    );
}
