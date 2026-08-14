//! Shared wrappers and helpers for `ai-interface` model implementations.

#![warn(unreachable_pub)]

mod catalog;
mod concurrency;
mod errors;
mod polling;
mod pricing;
mod retrying;
mod sleeper;
mod tool_call_identity;

pub use catalog::{
    CostTier, IntelligenceScore, KnownModelCatalog, KnownModelSpec, ModelFeature, ProviderKind,
    SpeedTier, ThinkingLevel, known_mock_models, resolve_catalog_thinking_level,
};
pub use concurrency::ConcurrencyLimitedModel;
pub use errors::{
    assistant_text, classify_json_http_error, parse_structured_output, parse_tool_call_arguments,
    validate_structured_output,
};
#[cfg(any(test, doctest, feature = "test-support"))]
pub use polling::PollingRuntimeMock;
pub use polling::{DynPollingRuntime, PollingRuntime, TokioPollingRuntime};
pub use pricing::{ModelPricing, UsagePricingModel, price_usage};
pub use retrying::{RetryingModel, STANDARD_TRANSIENT_RETRY_DELAYS};
#[cfg(any(test, doctest))]
pub use sleeper::SleeperMock;
pub use sleeper::{DynSleeper, Sleeper, TokioSleeper};
pub use tool_call_identity::{synthetic_tool_call_id, synthetic_tool_call_scope};

#[cfg(test)]
#[path = "_tests_/mod.rs"]
mod tests;
