use std::sync::Arc;

use ai_interface::{
    FinishReason, Model, ModelGenerationControls, ModelMock, ModelRequest, ModelResponse,
    ModelUsage, ModelUsageMeasurementState, ModelUsageUnitKind,
};
use unimock::{MockFn, Unimock, matching};

use crate::{ModelPricing, UsagePricingModel, price_usage};

#[tokio::test]
async fn pricing_wrapper_forwards_call_controls_unchanged() {
    let inner: Arc<dyn Model> = Arc::new(Unimock::new(
        ModelMock::complete
            .next_call(matching!(_))
            .answers(&|_, request: &ModelRequest| {
                assert_eq!(request.controls.generation.max_output_tokens, Some(4321));
                Ok(success_response())
            }),
    ));
    let model = UsagePricingModel::new(inner, ModelPricing::default());
    let mut request = empty_request();
    request.controls.generation = ModelGenerationControls {
        max_output_tokens: Some(4321),
        ..Default::default()
    };

    model
        .complete(&request)
        .await
        .expect("pricing wrapper should succeed");
}

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

fn empty_request() -> ModelRequest {
    ModelRequest {
        system_prompt: "system".to_owned(),
        messages: Vec::new(),
        tools: Vec::new(),
        response_schema: None,
        controls: Default::default(),
    }
}

fn success_response() -> ModelResponse {
    ModelResponse {
        provider: "mock".to_owned(),
        model_id: "mock".to_owned(),
        catalog_model_id: None,
        thinking_level: None,
        assistant_message: "ok".to_owned(),
        tool_calls: Vec::new(),
        finish_reason: FinishReason::Stop,
        structured_output: None,
        provider_context: Vec::new(),
        usage: ModelUsage::default(),
    }
}
