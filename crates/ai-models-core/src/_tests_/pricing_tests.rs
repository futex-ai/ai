use ai_interface::{ModelUsage, ModelUsageMeasurementState, ModelUsageUnitKind};

use crate::{ModelPricing, price_usage};

#[test]
fn price_usage_calculates_measured_line_costs() {
    let usage = ModelUsage {
        input_tokens: 1_500_000,
        output_tokens: 500_000,
        cached_input_tokens: 10,
        reasoning_tokens: 0,
        total_tokens: 2_000_000,
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
            reasoning_token_usd_micros_per_million: None,
            free_when_unpriced: false,
        },
    );

    assert_eq!(priced.estimated_cost_microusd, 3_000_001);
    assert_eq!(priced.cost_lines.len(), 3);
    assert_eq!(
        priced.cost_lines[0].unit_kind,
        ModelUsageUnitKind::InputToken
    );
    assert_eq!(
        priced.cost_lines[0].measurement_state,
        ModelUsageMeasurementState::Measured
    );
    assert_eq!(priced.cost_lines[0].cost_usd_micros, Some(1_500_000));
}

#[test]
fn price_usage_prices_non_overlapping_cached_and_reasoning_buckets() {
    let priced = price_usage(
        ModelUsage {
            input_tokens: 80_000_000,
            output_tokens: 20_000_000,
            cached_input_tokens: 40_000_000,
            reasoning_tokens: 12_000_000,
            total_tokens: 152_000_000,
            estimated_cost_microusd: 0,
            cost_lines: Vec::new(),
        },
        &ModelPricing {
            rate_version: Some("2026-06-14".to_owned()),
            input_token_usd_micros_per_million: Some(1),
            output_token_usd_micros_per_million: Some(10),
            cached_input_token_usd_micros_per_million: Some(100),
            reasoning_token_usd_micros_per_million: Some(1_000),
            free_when_unpriced: false,
        },
    );

    assert_eq!(priced.estimated_cost_microusd, 16_280);
    assert_eq!(priced.cost_lines.len(), 4);
}

#[test]
fn price_usage_marks_unpriced_usage_unknown() {
    let priced = price_usage(
        ModelUsage {
            input_tokens: 100,
            output_tokens: 0,
            cached_input_tokens: 0,
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
