//! Normalized model usage and cost-line DTOs.

use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
/// Usage and estimated cost information returned by a model call.
pub struct ModelUsage {
    /// Non-cached input token count used for regular input-token pricing.
    pub input_tokens: u64,
    /// Non-reasoning output token count used for regular output-token pricing.
    pub output_tokens: u64,
    /// Cached input token count, when separated by the provider.
    #[serde(default)]
    pub cached_input_tokens: u64,
    /// Reasoning token count, when separated by the provider.
    #[serde(default)]
    pub reasoning_tokens: u64,
    /// Provider-reported total token count, or the normalized total when absent.
    pub total_tokens: u64,
    /// Estimated request cost in micro-USD when provided.
    pub estimated_cost_microusd: u64,
    /// Priced line items used by workspace metering.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub cost_lines: Vec<ModelUsageCostLine>,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
/// Billable usage unit kinds normalized from provider responses.
pub enum ModelUsageUnitKind {
    /// Model input token.
    InputToken,
    /// Model output token.
    OutputToken,
    /// Cached input token.
    CachedInputToken,
    /// Reasoning token.
    ReasoningToken,
}

impl ModelUsageUnitKind {
    /// Returns the stable snake-case value used in storage and APIs.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::InputToken => "input_token",
            Self::OutputToken => "output_token",
            Self::CachedInputToken => "cached_input_token",
            Self::ReasoningToken => "reasoning_token",
        }
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
/// Measurement quality for one normalized usage line.
pub enum ModelUsageMeasurementState {
    /// Quantity and price are known from provider usage and configured pricing.
    Measured,
    /// Quantity or price was estimated by the workspace.
    Estimated,
    /// Quantity is known but no configured price exists.
    Unknown,
    /// Usage is intentionally free.
    Free,
}

impl ModelUsageMeasurementState {
    /// Returns the stable snake-case value used in storage and APIs.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Measured => "measured",
            Self::Estimated => "estimated",
            Self::Unknown => "unknown",
            Self::Free => "free",
        }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
/// One normalized and optionally priced model usage line.
pub struct ModelUsageCostLine {
    /// Billable unit kind.
    pub unit_kind: ModelUsageUnitKind,
    /// Provider-reported or estimated quantity.
    pub quantity: u64,
    /// Unit price in micro-USD per one million units, when known.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub unit_price_usd_micros_per_million: Option<u64>,
    /// Cost for this line in micro-USD, when known.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cost_usd_micros: Option<u64>,
    /// Version of the pricing rule used to calculate this line.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rate_version: Option<String>,
    /// Measurement quality for this line.
    pub measurement_state: ModelUsageMeasurementState,
}
