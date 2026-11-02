//! Test construction adapter for the shared request mapper.

use super::dialect::DIALECT;
use crate::engine::{request, request_types::ChatCompletionsRequest};
use ai_interface::{ModelRequest, ModelResult};
use ai_models_core::ThinkingLevel;

pub(super) fn build_request(
    model_id: &str,
    thinking: ThinkingLevel,
    input: &ModelRequest,
) -> ModelResult<ChatCompletionsRequest> {
    request::build_request(&DIALECT, model_id, thinking, input)
}
