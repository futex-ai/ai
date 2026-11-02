//! Test entry point for request mapping with the public spec resolver.

use ai_interface::{Effort, ModelRequest, ModelResult, ModelSpec};
use ai_models_core::resolve_controls;

use crate::engine::{request, request_types::ChatCompletionsRequest};

use super::{CATALOG, CUSTOM_DEFAULTS, PROVIDER_ID, dialect::DIALECT};

pub(super) fn build_request(
    model_id: &str,
    effort: Effort,
    input: &ModelRequest,
) -> ModelResult<ChatCompletionsRequest> {
    let spec = CATALOG
        .iter()
        .find(|spec| spec.provider_model_id == model_id)
        .cloned()
        .unwrap_or_else(|| ModelSpec::custom(PROVIDER_ID, model_id.to_owned(), &CUSTOM_DEFAULTS));
    let mut input = input.clone();
    input.controls.generation.effort = Some(effort);
    let controls = resolve_controls(&input, &spec)?;
    request::build_request(&DIALECT, model_id, &input, &controls)
}
