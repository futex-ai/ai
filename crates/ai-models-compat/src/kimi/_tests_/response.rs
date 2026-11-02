//! Test construction adapter for the shared response mapper.

use super::dialect::DIALECT;
use crate::engine::{CompletionContext, response};
use ai_interface::{ModelResponse, ModelResult, StructuredOutputSchema};
use ai_models_core::ThinkingLevel;
use serde_json::Value;

pub(super) fn parse_response(
    catalog_model_id: &str,
    provider_model_id: &str,
    thinking_level: ThinkingLevel,
    body: Value,
    schema: Option<&StructuredOutputSchema>,
) -> ModelResult<ModelResponse> {
    response::parse_response(
        CompletionContext {
            dialect: &DIALECT,
            catalog_model_id,
            provider_model_id,
            thinking_level,
        },
        body,
        schema,
    )
}
