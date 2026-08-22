//! Shared wrappers and helpers for `ai-interface` model implementations.

#![warn(unreachable_pub)]

mod catalog;
mod chat_completions;
mod concurrency;
mod errors;
mod polling;
mod pricing;
mod retrying;
mod sleeper;
mod tool_call_identity;

#[cfg(any(test, feature = "test-support"))]
pub mod test_support;

pub use catalog::{
    CostTier, IntelligenceScore, KnownModelCatalog, KnownModelSpec, ModelFeature, ProviderKind,
    SpeedTier, ThinkingLevel, known_mock_models, resolve_catalog_thinking_level,
};
pub use chat_completions::{
    ChatCompletionsAccumulator, ChatCompletionsChoice, ChatCompletionsCompletionTokenDetails,
    ChatCompletionsDelta, ChatCompletionsMessage, ChatCompletionsPromptTokenDetails,
    ChatCompletionsResponse, ChatCompletionsStreamError, ChatCompletionsStreamStatus,
    ChatCompletionsStreamUpdate, ChatCompletionsToolCall, ChatCompletionsToolFunction,
    ChatCompletionsUsage,
};
pub use concurrency::ConcurrencyLimitedModel;
pub use errors::{
    assistant_text, classify_chat_completions_provider_error, classify_json_http_error,
    classify_json_http_stream_error, classify_stream_error, parse_structured_output,
    parse_tool_call_arguments, validate_structured_output,
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
