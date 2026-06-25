//! Shared wrappers and helpers for `ai-interface` model implementations.

#![warn(unreachable_pub)]

mod catalog;
mod concurrency;
mod errors;
mod pricing;
mod retrying;
mod sleeper;

pub use catalog::{
    CostTier, IntelligenceScore, KnownModelCatalog, KnownModelSpec, ModelFeature, ProviderKind,
    SpeedTier, ThinkingLevel, known_mock_models,
};
pub use concurrency::ConcurrencyLimitedModel;
pub use errors::{
    assistant_text, classify_json_http_error, parse_structured_output, parse_tool_call_arguments,
    validate_structured_output,
};
pub use pricing::{ModelPricing, UsagePricingModel, price_usage};
pub use retrying::{RetryingModel, STANDARD_TRANSIENT_RETRY_DELAYS};
#[cfg(any(test, doctest))]
pub use sleeper::SleeperMock;
pub use sleeper::{DynSleeper, Sleeper, TokioSleeper};

#[cfg(test)]
#[path = "_tests_/mod.rs"]
mod tests;
