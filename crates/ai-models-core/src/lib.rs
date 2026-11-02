//! Provider helpers for `ai-interface` model implementations.

#![warn(unreachable_pub)]

mod capabilities;
mod capturing_auth;
mod catalog;
mod clock;
mod error_context;
mod error_metadata;
mod errors;
mod http_status;
mod polling;
mod provider_auth;
mod provider_config;
mod structured_output;
mod tool_call_identity;

#[cfg(any(test, feature = "test-support"))]
pub mod test_support;

pub use ai_interface::{CostTier, IntelligenceScore, ModelFeature, SpeedTier};
pub use capabilities::{ResolvedModelControls, ResolvedToolChoice, resolve_controls};
pub use catalog::known_mock_models;
#[cfg(any(test, doctest, feature = "test-support"))]
pub use clock::ClockMock;
pub use clock::{Clock, SystemClock};
pub use error_context::ErrorContext;
pub use error_metadata::{ErrorMetadata, parse_retry_after};
pub use errors::{
    assistant_text, classify_json_http_error, classify_json_http_stream_error,
    classify_stream_error, json_http_error_core,
};
pub use http_status::classify_http_status;
#[cfg(any(test, doctest, feature = "test-support"))]
pub use polling::PollingRuntimeMock;
pub use polling::{DynPollingRuntime, PollingRuntime, TokioPollingRuntime};
pub use provider_auth::ApiKeyScheme;
pub use provider_config::{ProviderConfig, ProviderTimeouts};
pub use structured_output::{
    parse_structured_output, parse_tool_call_arguments, validate_structured_output,
};
pub use tool_call_identity::{synthetic_tool_call_id, synthetic_tool_call_scope};

#[cfg(test)]
#[path = "_tests_/mod.rs"]
mod tests;
