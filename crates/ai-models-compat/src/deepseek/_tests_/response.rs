//! Test construction adapter for the shared response mapper.

use ai_interface::{Effort, ModelResponse, ModelResult, StructuredOutputSchema};
use ai_models_core::{ErrorContext, SystemClock};
use serde_json::Value;
use std::sync::Arc;

use crate::engine::{CompletionContext, response};

use super::dialect::DIALECT;

pub(super) fn parse_response(
    catalog_model_id: &str,
    provider_model_id: &str,
    thinking_level: Effort,
    body: Value,
    schema: Option<&StructuredOutputSchema>,
) -> ModelResult<ModelResponse> {
    let error = ErrorContext::new("deepseek", provider_model_id, &[], Arc::new(SystemClock));
    response::parse_response(
        CompletionContext {
            dialect: &DIALECT,
            error: &error,
            catalog_model_id,
            provider_model_id,
            thinking_level: Some(thinking_level),
            synthetic_tool_call_scope: "",
        },
        body,
        schema,
    )
}
