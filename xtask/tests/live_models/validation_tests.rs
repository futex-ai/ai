//! Normalized response checks shared by live chat probes.

use ai_interface::{Effort, FinishReason, ModelResponse, ModelSpec};

use super::{
    provider_tests::LiveProvider,
    runner_tests::{EXPECTED_TEXT, MARKER_MISS_FAILURE},
};

/// Adds every normalized response failure for the selected probe.
pub(super) fn validate_response(
    provider: LiveProvider,
    spec: &ModelSpec,
    effort: Effort,
    response: &ModelResponse,
    failures: &mut Vec<String>,
) {
    let expected_provider = provider.provider_id();
    if response.provider != expected_provider.as_str() {
        failures.push(format!(
            "{}: provider was `{}`, expected `{expected_provider}`",
            spec.text_id(),
            response.provider
        ));
    }
    if response.model_id != spec.provider_model_id {
        failures.push(format!(
            "{}: provider model was `{}`, expected `{}`",
            spec.text_id(),
            response.model_id,
            spec.provider_model_id
        ));
    }
    if response.catalog_model_id.as_deref() != Some(spec.text_id().as_str()) {
        failures.push(format!(
            "{}: catalog model was {:?}",
            spec.text_id(),
            response.catalog_model_id
        ));
    }
    if response.effort() != Some(effort) {
        failures.push(format!(
            "{}: effort was {:?}, expected `{}`",
            spec.text_id(),
            response.effort(),
            effort.as_str()
        ));
    }
    if !matches!(response.finish_reason, FinishReason::Stop) {
        failures.push(format!(
            "{}: finish reason was {:?}",
            spec.text_id(),
            response.finish_reason
        ));
    }
    if !response.text().contains(EXPECTED_TEXT) {
        failures.push(format!("{}: {MARKER_MISS_FAILURE}", spec.text_id()));
    }
    if !response.tool_calls().is_empty() {
        failures.push(format!(
            "{}: response unexpectedly requested tools",
            spec.text_id()
        ));
    }
    if response.usage.total_tokens == 0 {
        failures.push(format!(
            "{}: provider reported zero total tokens",
            spec.text_id()
        ));
    }
}
