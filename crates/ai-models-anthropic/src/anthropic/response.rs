//! Anthropic messages response parsing.

use ai_interface::{
    FinishReason, ModelError, ModelResponse, ModelUsage, StructuredOutputSchema, ToolCall,
};
use ai_models_core::{ThinkingLevel, parse_structured_output};
use serde::Deserialize;
use serde_json::Value;

const PROVIDER: &str = "anthropic";

pub(super) fn parse_response(
    catalog_model_id: &str,
    provider_model_id: &str,
    thinking_level: ThinkingLevel,
    body: Value,
    response_schema: Option<&StructuredOutputSchema>,
) -> std::result::Result<ModelResponse, ModelError> {
    let parsed: AnthropicResponse = serde_json::from_value(body).map_err(|source| {
        ModelError::provider(
            PROVIDER,
            provider_model_id,
            format!("invalid Anthropic response: {source}"),
        )
    })?;
    let mut assistant_parts = Vec::new();
    let mut parsed_tool_calls = Vec::new();

    for block in parsed.content {
        match block {
            AnthropicContentBlock::Text { text } => {
                if !text.trim().is_empty() {
                    assistant_parts.push(text);
                }
            }
            AnthropicContentBlock::ToolUse { id, name, input } => {
                parsed_tool_calls.push(ToolCall {
                    id,
                    name,
                    input,
                    operation_id: None,
                });
            }
            AnthropicContentBlock::Ignored => {}
        }
    }

    let assistant_message = assistant_parts.join("\n");
    let finish_reason = finish_reason(parsed.stop_reason.as_deref());
    let tool_calls = if matches!(finish_reason, FinishReason::ToolCalls) {
        parsed_tool_calls
    } else {
        Vec::new()
    };
    let structured_output = response_schema
        .filter(|_| matches!(finish_reason, FinishReason::Stop) && tool_calls.is_empty())
        .map(|response_schema| {
            parse_structured_output(
                PROVIDER,
                provider_model_id,
                &assistant_message,
                response_schema,
            )
        })
        .transpose()?;
    let assistant_message = if assistant_message.trim().is_empty() {
        structured_output
            .as_ref()
            .map(Value::to_string)
            .unwrap_or_default()
    } else {
        assistant_message
    };
    let input_tokens = u64::from(parsed.usage.input_tokens)
        .saturating_add(u64::from(parsed.usage.cache_creation_input_tokens));
    let output_tokens = u64::from(parsed.usage.output_tokens);
    let cached_input_tokens = u64::from(parsed.usage.cache_read_input_tokens);
    Ok(ModelResponse {
        provider: PROVIDER.to_owned(),
        model_id: provider_model_id.to_owned(),
        catalog_model_id: Some(catalog_model_id.to_owned()),
        thinking_level: Some(thinking_level.as_str().to_owned()),
        assistant_message,
        tool_calls,
        finish_reason,
        structured_output,
        provider_context: Vec::new(),
        usage: ModelUsage {
            input_tokens,
            output_tokens,
            cached_input_tokens,
            reasoning_tokens: 0,
            total_tokens: input_tokens
                .saturating_add(output_tokens)
                .saturating_add(cached_input_tokens),
            estimated_cost_microusd: 0,
            cost_lines: Vec::new(),
        },
    })
}

#[derive(Debug, Deserialize)]
struct AnthropicResponse {
    content: Vec<AnthropicContentBlock>,
    #[serde(default)]
    stop_reason: Option<String>,
    #[serde(default)]
    usage: AnthropicUsage,
}

#[derive(Clone, Debug, Default, Deserialize)]
struct AnthropicUsage {
    #[serde(default)]
    input_tokens: u32,
    #[serde(default)]
    output_tokens: u32,
    #[serde(default)]
    cache_read_input_tokens: u32,
    #[serde(default)]
    cache_creation_input_tokens: u32,
}

#[derive(Debug, Deserialize)]
#[serde(tag = "type")]
enum AnthropicContentBlock {
    #[serde(rename = "text")]
    Text { text: String },
    #[serde(rename = "tool_use")]
    ToolUse {
        id: String,
        name: String,
        input: Value,
    },
    #[serde(other)]
    Ignored,
}

fn finish_reason(value: Option<&str>) -> FinishReason {
    match value {
        Some("end_turn" | "stop_sequence") => FinishReason::Stop,
        Some("tool_use") => FinishReason::ToolCalls,
        Some("max_tokens" | "model_context_window_exceeded") => FinishReason::Truncated,
        Some("refusal") => FinishReason::Filtered,
        Some(raw) => FinishReason::Other(raw.to_owned()),
        None => FinishReason::Other("missing".to_owned()),
    }
}
