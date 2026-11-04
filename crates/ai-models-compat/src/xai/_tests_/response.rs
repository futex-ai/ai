//! Test construction adapter for the shared response mapper.

use ai_interface::{Effort, ModelResponse, ModelResult, StructuredOutputSchema};
use serde_json::Value;

use crate::engine::{CompletionContext, response};

use super::dialect::DIALECT;

pub(super) fn parse_response(
    catalog_model_id: &str,
    provider_model_id: &str,
    thinking_level: Effort,
    synthetic_tool_call_scope: &str,
    body: Value,
    schema: Option<&StructuredOutputSchema>,
) -> ModelResult<ModelResponse> {
    response::parse_response(
        CompletionContext {
            dialect: &DIALECT,
            catalog_model_id,
            provider_model_id,
            thinking_level: Some(thinking_level),
            synthetic_tool_call_scope,
        },
        body,
        schema,
    )
}
