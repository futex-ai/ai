//! OpenAI Decisions usage normalization.

use ai_interface::ModelUsage;
use serde::Deserialize;

/// Provider usage counters, each of which may be absent or `null`.
#[derive(Debug, Default, Deserialize)]
pub(super) struct DecisionsUsage {
    #[serde(default)]
    input_tokens: Option<u64>,
    #[serde(default)]
    input_tokens_details: Option<InputTokensDetails>,
    #[serde(default)]
    output_tokens: Option<u64>,
    #[serde(default)]
    output_tokens_details: Option<OutputTokensDetails>,
    #[serde(default)]
    total_tokens: Option<u64>,
}

#[derive(Debug, Default, Deserialize)]
struct InputTokensDetails {
    #[serde(default)]
    cached_tokens: Option<u64>,
}

#[derive(Debug, Default, Deserialize)]
struct OutputTokensDetails {
    #[serde(default)]
    reasoning_tokens: Option<u64>,
}

/// Normalizes Decisions usage with the OpenAI Responses conventions.
///
/// Cached input and reasoning tokens are separated from the regular input and
/// output buckets with saturating arithmetic. The provider total is kept when
/// present; otherwise it is the saturating sum of the four buckets.
pub(super) fn normalize_usage(usage: Option<DecisionsUsage>) -> ModelUsage {
    let Some(usage) = usage else {
        return ModelUsage::default();
    };
    let cached_input_tokens = usage
        .input_tokens_details
        .and_then(|details| details.cached_tokens)
        .unwrap_or_default();
    let reasoning_tokens = usage
        .output_tokens_details
        .and_then(|details| details.reasoning_tokens)
        .unwrap_or_default();
    let input_tokens = usage
        .input_tokens
        .unwrap_or_default()
        .saturating_sub(cached_input_tokens);
    let output_tokens = usage
        .output_tokens
        .unwrap_or_default()
        .saturating_sub(reasoning_tokens);
    let total_tokens = usage.total_tokens.unwrap_or_else(|| {
        input_tokens
            .saturating_add(cached_input_tokens)
            .saturating_add(output_tokens)
            .saturating_add(reasoning_tokens)
    });
    ModelUsage {
        input_tokens,
        output_tokens,
        cached_input_tokens,
        reasoning_tokens,
        total_tokens,
        ..ModelUsage::default()
    }
}

#[cfg(test)]
#[path = "_tests_/usage_tests.rs"]
mod usage_tests;
