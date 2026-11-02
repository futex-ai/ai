//! Test construction adapter for the shared request mapper.

use super::{client::KimiReasoningEffort, dialect::DIALECT};
use crate::engine::{request, request_types::ChatCompletionsRequest};
use ai_interface::{ModelRequest, ModelResult};

pub(super) fn build_request(
    model_id: &str,
    effort: KimiReasoningEffort,
    input: &ModelRequest,
) -> ModelResult<ChatCompletionsRequest> {
    request::build_request(&DIALECT, model_id, effort.thinking_level(), input)
}
