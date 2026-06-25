//! Built-in mock model for development and tests.

use async_trait::async_trait;
use serde_json::{Map, Value};

use crate::{
    ConversationRole, FinishReason, Model, ModelError, ModelRequest, ModelResponse, ModelResult,
    ModelUsage,
};

/// Simple deterministic mock model used by development and tests.
#[derive(Clone, Debug)]
pub struct MockModel {
    model_id: String,
    provider: String,
}

impl MockModel {
    /// Builds a mock model that reports the provided concrete model identifier.
    pub fn new(model_id: impl Into<String>) -> Self {
        Self {
            model_id: model_id.into(),
            provider: "mock".to_owned(),
        }
    }

    /// Builds a mock model with explicit provider and model identifiers.
    pub fn with_provider(provider: impl Into<String>, model_id: impl Into<String>) -> Self {
        Self {
            model_id: model_id.into(),
            provider: provider.into(),
        }
    }
}

impl Default for MockModel {
    fn default() -> Self {
        Self::new("mock")
    }
}

#[async_trait]
impl Model for MockModel {
    async fn complete(&self, request: &ModelRequest) -> ModelResult<ModelResponse> {
        let subject = extract_mock_subject(request);
        let fallback_message = format!("Acknowledged: {subject}");
        let structured_output = match request.response_schema.as_ref() {
            Some(response_schema) => Some(
                mock_structured_output(&response_schema.schema, &fallback_message).ok_or_else(
                    || {
                        ModelError::provider(
                            &self.provider,
                            &self.model_id,
                            format!(
                                "mock model could not synthesize structured output for schema `{}`",
                                response_schema.name
                            ),
                        )
                    },
                )?,
            ),
            None => None,
        };
        let assistant_message = structured_output
            .as_ref()
            .map(Value::to_string)
            .unwrap_or(fallback_message);
        let input_tokens = estimate_tokens(
            std::iter::once(request.system_prompt.as_str()).chain(
                request
                    .messages
                    .iter()
                    .map(|message| message.content.as_str()),
            ),
        );
        let output_tokens = estimate_tokens(std::iter::once(assistant_message.as_str()));
        Ok(ModelResponse {
            provider: self.provider.clone(),
            model_id: self.model_id.clone(),
            catalog_model_id: None,
            thinking_level: None,
            assistant_message,
            tool_calls: Vec::new(),
            finish_reason: FinishReason::Stop,
            structured_output,
            usage: ModelUsage {
                input_tokens: u64::from(input_tokens),
                output_tokens: u64::from(output_tokens),
                cached_input_tokens: 0,
                reasoning_tokens: 0,
                total_tokens: u64::from(input_tokens.saturating_add(output_tokens)),
                estimated_cost_microusd: 0,
                cost_lines: Vec::new(),
            },
        })
    }
}

fn extract_mock_subject(request: &ModelRequest) -> String {
    let Some(message) = request
        .messages
        .iter()
        .rev()
        .find(|message| message.role == ConversationRole::User)
    else {
        return "no current task".to_owned();
    };

    for line in message.content.lines() {
        if let Some(body) = line.trim().strip_prefix("- body: ") {
            return body.trim().to_owned();
        }
        if let Some(body) = line.trim().strip_prefix("- body_json: ") {
            return body.trim().to_owned();
        }
    }

    message
        .content
        .lines()
        .rev()
        .find_map(|line| {
            let trimmed = line.trim();
            (!trimmed.is_empty()).then(|| trimmed.to_owned())
        })
        .unwrap_or_else(|| "no current task".to_owned())
}

fn estimate_tokens<'a>(segments: impl IntoIterator<Item = &'a str>) -> u32 {
    segments
        .into_iter()
        .map(|segment| segment.split_whitespace().count() as u32)
        .sum()
}

fn mock_structured_output(schema: &Value, message: &str) -> Option<Value> {
    if let Some(value) = schema.get("const") {
        return Some(value.clone());
    }
    if let Some(value) = schema.get("default") {
        return Some(value.clone());
    }
    if let Some(values) = schema.get("enum").and_then(Value::as_array) {
        return values.first().cloned();
    }
    if let Some(variants) = schema
        .get("anyOf")
        .and_then(Value::as_array)
        .or_else(|| schema.get("oneOf").and_then(Value::as_array))
    {
        return variants
            .iter()
            .find_map(|variant| mock_structured_output(variant, message));
    }
    if matches_type(schema, "object") || schema.get("properties").is_some() {
        let mut output = Map::new();
        if let Some(properties) = schema.get("properties").and_then(Value::as_object) {
            for (key, property_schema) in properties {
                if let Some(value) = mock_structured_output(property_schema, message) {
                    output.insert(key.clone(), value);
                }
            }
        }
        return Some(Value::Object(output));
    }
    if matches_type(schema, "array") || schema.get("items").is_some() {
        let item_count = schema
            .get("minItems")
            .and_then(Value::as_u64)
            .unwrap_or(1)
            .max(1) as usize;
        let item = schema
            .get("items")
            .and_then(|item_schema| mock_structured_output(item_schema, message));
        return Some(Value::Array(
            item.map(|value| vec![value; item_count])
                .unwrap_or_default(),
        ));
    }
    if matches_type(schema, "string") {
        return Some(Value::String(message.to_owned()));
    }
    if matches_type(schema, "integer") {
        return Some(Value::from(0));
    }
    if matches_type(schema, "number") {
        return Some(Value::from(0.0));
    }
    if matches_type(schema, "boolean") {
        return Some(Value::Bool(true));
    }
    if matches_type(schema, "null") {
        return Some(Value::Null);
    }
    None
}

fn matches_type(schema: &Value, expected: &str) -> bool {
    match schema.get("type") {
        Some(Value::String(value)) => value == expected,
        Some(Value::Array(values)) => values.iter().any(|value| value.as_str() == Some(expected)),
        _ => false,
    }
}
