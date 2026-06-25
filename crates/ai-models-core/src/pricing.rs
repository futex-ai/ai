//! Model usage pricing wrapper.

use std::sync::Arc;

use ai_interface::{
    Model, ModelRequest, ModelResponse, ModelResult, ModelUsage, ModelUsageCostLine,
    ModelUsageMeasurementState, ModelUsageUnitKind,
};
use async_trait::async_trait;

#[derive(Clone, Debug, Default, Eq, PartialEq)]
/// Prices used to calculate model usage cost at call time.
pub struct ModelPricing {
    /// Version or effective-date label copied into metering rows.
    pub rate_version: Option<String>,
    /// Input-token price in micro-USD per one million tokens.
    pub input_token_usd_micros_per_million: Option<u64>,
    /// Output-token price in micro-USD per one million tokens.
    pub output_token_usd_micros_per_million: Option<u64>,
    /// Cached-input-token price in micro-USD per one million tokens.
    pub cached_input_token_usd_micros_per_million: Option<u64>,
    /// Reasoning-token price in micro-USD per one million tokens.
    pub reasoning_token_usd_micros_per_million: Option<u64>,
    /// Whether usage with no explicit price should be treated as free.
    pub free_when_unpriced: bool,
}

impl ModelPricing {
    /// Returns a pricing policy that records zero-cost free usage.
    pub fn free(rate_version: impl Into<String>) -> Self {
        Self {
            rate_version: Some(rate_version.into()),
            input_token_usd_micros_per_million: Some(0),
            output_token_usd_micros_per_million: Some(0),
            cached_input_token_usd_micros_per_million: Some(0),
            reasoning_token_usd_micros_per_million: Some(0),
            free_when_unpriced: true,
        }
    }
}

#[derive(Clone)]
/// Model wrapper that calculates cost lines for successful responses.
pub struct UsagePricingModel {
    inner: Arc<dyn Model>,
    pricing: ModelPricing,
}

impl UsagePricingModel {
    /// Builds a pricing wrapper around another model implementation.
    pub fn new(inner: Arc<dyn Model>, pricing: ModelPricing) -> Self {
        Self { inner, pricing }
    }
}

#[async_trait]
impl Model for UsagePricingModel {
    async fn complete(&self, request: &ModelRequest) -> ModelResult<ModelResponse> {
        let mut response = self.inner.complete(request).await?;
        response.usage = price_usage(response.usage, &self.pricing);
        Ok(response)
    }
}

/// Applies configured pricing to a model usage payload.
pub fn price_usage(mut usage: ModelUsage, pricing: &ModelPricing) -> ModelUsage {
    let mut lines = Vec::new();
    push_line(
        &mut lines,
        ModelUsageUnitKind::InputToken,
        usage.input_tokens,
        pricing.input_token_usd_micros_per_million,
        pricing,
    );
    push_line(
        &mut lines,
        ModelUsageUnitKind::OutputToken,
        usage.output_tokens,
        pricing.output_token_usd_micros_per_million,
        pricing,
    );
    push_line(
        &mut lines,
        ModelUsageUnitKind::CachedInputToken,
        usage.cached_input_tokens,
        pricing.cached_input_token_usd_micros_per_million,
        pricing,
    );
    push_line(
        &mut lines,
        ModelUsageUnitKind::ReasoningToken,
        usage.reasoning_tokens,
        pricing.reasoning_token_usd_micros_per_million,
        pricing,
    );
    usage.estimated_cost_microusd = lines
        .iter()
        .filter_map(|line| line.cost_usd_micros)
        .sum::<u64>();
    usage.cost_lines = lines;
    usage
}

fn push_line(
    lines: &mut Vec<ModelUsageCostLine>,
    unit_kind: ModelUsageUnitKind,
    quantity: u64,
    price: Option<u64>,
    pricing: &ModelPricing,
) {
    if quantity == 0 {
        return;
    }
    let state = measurement_state(price, pricing.free_when_unpriced);
    let effective_price = price.or_else(|| pricing.free_when_unpriced.then_some(0));
    let cost = effective_price.map(|value| cost_usd_micros(quantity, value));
    lines.push(ModelUsageCostLine {
        unit_kind,
        quantity,
        unit_price_usd_micros_per_million: effective_price,
        cost_usd_micros: cost,
        rate_version: pricing.rate_version.clone(),
        measurement_state: state,
    });
}

fn measurement_state(price: Option<u64>, free_when_unpriced: bool) -> ModelUsageMeasurementState {
    match (price, free_when_unpriced) {
        (Some(0), _) | (None, true) => ModelUsageMeasurementState::Free,
        (Some(_), _) => ModelUsageMeasurementState::Measured,
        (None, false) => ModelUsageMeasurementState::Unknown,
    }
}

fn cost_usd_micros(quantity: u64, price_usd_micros_per_million: u64) -> u64 {
    let product = u128::from(quantity) * u128::from(price_usd_micros_per_million);
    let rounded = (product + 500_000) / 1_000_000;
    rounded.try_into().unwrap_or(u64::MAX)
}

#[cfg(test)]
#[path = "_tests_/pricing_tests.rs"]
mod pricing_tests;
