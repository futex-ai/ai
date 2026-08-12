use ai_interface::{ModelUsage, ModelUsageMeasurementState, ModelUsageUnitKind};

use crate::{ModelPricing, price_usage};

#[test]
fn price_usage_calculates_measured_line_costs() {
    let usage = ModelUsage {
        input_tokens: 1_500_000,
        output_tokens: 500_000,
        cached_input_tokens: 10,
        cache_write_input_tokens: 3,
        reasoning_tokens: 0,
        total_tokens: 2_000_013,
        estimated_cost_microusd: 0,
        cost_lines: Vec::new(),
    };
    let priced = price_usage(
        usage,
        &ModelPricing {
            rate_version: Some("2026-06-14".to_owned()),
            input_token_usd_micros_per_million: Some(1_000_000),
            output_token_usd_micros_per_million: Some(3_000_000),
            cached_input_token_usd_micros_per_million: Some(100_000),
            cache_write_input_token_usd_micros_per_million: Some(500_000),
            reasoning_token_usd_micros_per_million: None,
            free_when_unpriced: false,
        },
    );

    assert_eq!(priced.estimated_cost_microusd, 3_000_003);
    assert_eq!(priced.cost_lines.len(), 4);
    assert_eq!(
        priced.cost_lines[0].unit_kind,
        ModelUsageUnitKind::InputToken
    );
    assert_eq!(
        priced.cost_lines[0].measurement_state,
        ModelUsageMeasurementState::Measured
    );
    assert_eq!(priced.cost_lines[0].cost_usd_micros, Some(1_500_000));
    let cache_write_line = priced
        .cost_lines
        .iter()
        .find(|line| line.unit_kind == ModelUsageUnitKind::CacheWriteInputToken)
        .expect("cache-write line should exist");
    assert_eq!(cache_write_line.quantity, 3);
    assert_eq!(
        cache_write_line.unit_price_usd_micros_per_million,
        Some(500_000)
    );
    assert_eq!(cache_write_line.cost_usd_micros, Some(2));
    assert_eq!(
        cache_write_line.measurement_state,
        ModelUsageMeasurementState::Measured
    );
}

#[test]
fn price_usage_prices_non_overlapping_cached_and_reasoning_buckets() {
    let priced = price_usage(
        ModelUsage {
            input_tokens: 80_000_000,
            output_tokens: 20_000_000,
            cached_input_tokens: 40_000_000,
            cache_write_input_tokens: 5_000_000,
            reasoning_tokens: 12_000_000,
            total_tokens: 157_000_000,
            estimated_cost_microusd: 0,
            cost_lines: Vec::new(),
        },
        &ModelPricing {
            rate_version: Some("2026-06-14".to_owned()),
            input_token_usd_micros_per_million: Some(1),
            output_token_usd_micros_per_million: Some(10),
            cached_input_token_usd_micros_per_million: Some(100),
            cache_write_input_token_usd_micros_per_million: Some(500),
            reasoning_token_usd_micros_per_million: Some(1_000),
            free_when_unpriced: false,
        },
    );

    assert_eq!(priced.estimated_cost_microusd, 18_780);
    assert_eq!(priced.cost_lines.len(), 5);
}

#[test]
fn price_usage_marks_unpriced_usage_unknown() {
    let priced = price_usage(
        ModelUsage {
            input_tokens: 100,
            output_tokens: 0,
            cached_input_tokens: 0,
            cache_write_input_tokens: 0,
            reasoning_tokens: 0,
            total_tokens: 100,
            estimated_cost_microusd: 0,
            cost_lines: Vec::new(),
        },
        &ModelPricing::default(),
    );

    assert_eq!(priced.estimated_cost_microusd, 0);
    assert_eq!(priced.cost_lines[0].cost_usd_micros, None);
    assert_eq!(
        priced.cost_lines[0].measurement_state,
        ModelUsageMeasurementState::Unknown
    );
}

#[test]
fn price_usage_marks_unpriced_cache_writes_unknown() {
    let priced = price_usage(cache_write_usage(100), &ModelPricing::default());

    assert_eq!(priced.estimated_cost_microusd, 0);
    assert_eq!(priced.cost_lines.len(), 1);
    assert_eq!(
        priced.cost_lines[0].unit_kind,
        ModelUsageUnitKind::CacheWriteInputToken
    );
    assert_eq!(priced.cost_lines[0].unit_price_usd_micros_per_million, None);
    assert_eq!(priced.cost_lines[0].cost_usd_micros, None);
    assert_eq!(
        priced.cost_lines[0].measurement_state,
        ModelUsageMeasurementState::Unknown
    );
}

#[test]
fn price_usage_marks_unpriced_cache_writes_free_when_configured() {
    let priced = price_usage(
        cache_write_usage(100),
        &ModelPricing {
            free_when_unpriced: true,
            ..ModelPricing::default()
        },
    );

    assert_cache_write_line_is_free(&priced);
}

#[test]
fn free_pricing_marks_cache_writes_free() {
    let priced = price_usage(cache_write_usage(100), &ModelPricing::free("free"));

    assert_cache_write_line_is_free(&priced);
    assert_eq!(priced.cost_lines[0].rate_version.as_deref(), Some("free"));
}

#[test]
fn price_usage_omits_zero_quantity_cache_write_line() {
    let priced = price_usage(
        cache_write_usage(0),
        &ModelPricing {
            cache_write_input_token_usd_micros_per_million: Some(1_250_000),
            ..ModelPricing::default()
        },
    );

    assert!(priced.cost_lines.is_empty());
    assert_eq!(priced.estimated_cost_microusd, 0);
}

fn cache_write_usage(cache_write_input_tokens: u64) -> ModelUsage {
    ModelUsage {
        input_tokens: 0,
        output_tokens: 0,
        cached_input_tokens: 0,
        cache_write_input_tokens,
        reasoning_tokens: 0,
        total_tokens: cache_write_input_tokens,
        estimated_cost_microusd: 0,
        cost_lines: Vec::new(),
    }
}

fn assert_cache_write_line_is_free(priced: &ModelUsage) {
    assert_eq!(priced.estimated_cost_microusd, 0);
    assert_eq!(priced.cost_lines.len(), 1);
    assert_eq!(
        priced.cost_lines[0].unit_kind,
        ModelUsageUnitKind::CacheWriteInputToken
    );
    assert_eq!(
        priced.cost_lines[0].unit_price_usd_micros_per_million,
        Some(0)
    );
    assert_eq!(priced.cost_lines[0].cost_usd_micros, Some(0));
    assert_eq!(
        priced.cost_lines[0].measurement_state,
        ModelUsageMeasurementState::Free
    );
}
