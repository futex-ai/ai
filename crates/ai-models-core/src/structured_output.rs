//! Generated tool and schema output checks with safe per-call diagnostics.

use ai_interface::{ErrorKind, ModelError, ModelResult, StructuredOutputSchema};
use serde_json::Value;

use crate::ErrorContext;

/// Parses generated tool arguments and retains the original decoder source.
pub fn parse_tool_call_arguments(context: &ErrorContext, raw: &str) -> ModelResult<Value> {
    match serde_json::from_str(raw) {
        Ok(value) => Ok(value),
        Err(source) => Err(ModelError::with_source(
            context.core(
                ErrorKind::InvalidOutput,
                format!("invalid tool call JSON arguments `{raw}`: {source}"),
            ),
            source,
        )),
    }
}

/// Parses generated JSON text and checks it against the caller's schema.
pub fn parse_structured_output(
    context: &ErrorContext,
    assistant_message: &str,
    response_schema: &StructuredOutputSchema,
) -> ModelResult<Value> {
    let output = match serde_json::from_str(assistant_message) {
        Ok(output) => output,
        Err(source) => {
            return Err(ModelError::with_source(
                context.core(
                    ErrorKind::InvalidOutput,
                    format!(
                        "structured output for schema `{}` was not valid JSON: {source}",
                        response_schema.name
                    ),
                ),
                source,
            ));
        }
    };
    validate_structured_output(context, &output, response_schema)?;
    Ok(output)
}

/// Validates generated output while distinguishing invalid caller schemas.
pub fn validate_structured_output(
    context: &ErrorContext,
    structured_output: &Value,
    response_schema: &StructuredOutputSchema,
) -> ModelResult<()> {
    let validator = match jsonschema::validator_for(&response_schema.schema) {
        Ok(validator) => validator,
        Err(source) => {
            return Err(context.model_error(
                ErrorKind::InvalidRequest,
                format!(
                    "structured output schema `{}` could not be compiled: {source}",
                    response_schema.name
                ),
            ));
        }
    };
    match validator.validate(structured_output) {
        Ok(()) => Ok(()),
        Err(source) => Err(context.model_error(
            ErrorKind::InvalidOutput,
            format!(
                "structured output did not match schema `{}`: {source}",
                response_schema.name
            ),
        )),
    }
}
