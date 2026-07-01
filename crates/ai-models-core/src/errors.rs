//! Provider-agnostic error and JSON helper functions.

use ai_interface::{ModelError, ModelResult, StructuredOutputSchema};
use serde_json::Value;

/// Normalizes nullable assistant text content into a string.
pub fn assistant_text(content: Option<String>) -> String {
    content.unwrap_or_default()
}

/// Converts a JSON HTTP failure response into a typed model error.
pub fn classify_json_http_error(
    provider: &str,
    model_id: &str,
    status: u16,
    body: &Value,
) -> ModelError {
    let message = error_message_from_body(body).unwrap_or_else(|| format!("HTTP {status}"));
    if status == 400 && context_limit_code_from_body(body).is_some() {
        return ModelError::context_limit_exceeded(
            provider,
            model_id,
            format!("HTTP {status}: {message}"),
        );
    }
    if status == 429 {
        return ModelError::rate_limited(provider, model_id, format!("HTTP {status}: {message}"));
    }
    if status == 408 || status == 409 || status == 425 || (500..=599).contains(&status) {
        return ModelError::transient_provider(
            provider,
            model_id,
            format!("HTTP {status}: {message}"),
        );
    }
    ModelError::provider(provider, model_id, format!("HTTP {status}: {message}"))
}

fn context_limit_code_from_body(body: &Value) -> Option<&str> {
    let error = body.get("error").unwrap_or(body);
    let candidates = [
        error.get("code").and_then(Value::as_str),
        error.get("type").and_then(Value::as_str),
        error.get("status").and_then(Value::as_str),
        body.get("code").and_then(Value::as_str),
        body.get("status").and_then(Value::as_str),
    ];
    candidates.into_iter().flatten().find(|value| {
        matches!(
            *value,
            "context_length_exceeded"
                | "model_context_window_exceeded"
                | "input_too_long"
                | "too_many_tokens"
        )
    })
}

/// Parses raw tool-call arguments into structured JSON.
pub fn parse_tool_call_arguments(provider: &str, model_id: &str, raw: &str) -> ModelResult<Value> {
    serde_json::from_str(raw).map_err(|source| {
        ModelError::provider(
            provider,
            model_id,
            format!("invalid tool call JSON arguments `{raw}`: {source}"),
        )
    })
}

/// Parses assistant text into validated structured output.
pub fn parse_structured_output(
    provider: &str,
    model_id: &str,
    assistant_message: &str,
    response_schema: &StructuredOutputSchema,
) -> ModelResult<Value> {
    let structured_output = serde_json::from_str(assistant_message).map_err(|source| {
        ModelError::provider(
            provider,
            model_id,
            format!(
                "structured output for schema `{}` was not valid JSON: {source}",
                response_schema.name
            ),
        )
    })?;
    validate_structured_output(provider, model_id, &structured_output, response_schema)?;
    Ok(structured_output)
}

/// Validates structured output against the requested schema.
pub fn validate_structured_output(
    provider: &str,
    model_id: &str,
    structured_output: &Value,
    response_schema: &StructuredOutputSchema,
) -> ModelResult<()> {
    let validator = jsonschema::validator_for(&response_schema.schema).map_err(|source| {
        ModelError::provider(
            provider,
            model_id,
            format!(
                "structured output schema `{}` could not be compiled: {source}",
                response_schema.name
            ),
        )
    })?;
    validator.validate(structured_output).map_err(|source| {
        ModelError::provider(
            provider,
            model_id,
            format!(
                "structured output did not match schema `{}`: {source}",
                response_schema.name
            ),
        )
    })
}

fn error_message_from_body(body: &Value) -> Option<String> {
    if let Some(message) = body
        .get("error")
        .and_then(|error| error.get("message"))
        .and_then(Value::as_str)
    {
        return Some(message.to_owned());
    }
    body.get("message")
        .and_then(Value::as_str)
        .map(ToOwned::to_owned)
        .or_else(|| body.as_str().map(ToOwned::to_owned))
}
