//! Google Gemini response parsing.

use ai_interface::{
    FinishReason, ModelError, ModelResponse, ModelUsage, StructuredOutputSchema, ToolCall,
};
use ai_models_core::{ThinkingLevel, parse_structured_output};
use serde::Deserialize;
use serde_json::Value;

const PROVIDER: &str = "google";

pub(super) fn parse_response(
    catalog_model_id: &str,
    provider_model_id: &str,
    thinking_level: ThinkingLevel,
    body: Value,
    response_schema: Option<&StructuredOutputSchema>,
) -> std::result::Result<ModelResponse, ModelError> {
    let parsed: GoogleResponse = serde_json::from_value(body).map_err(|source| {
        ModelError::provider(
            PROVIDER,
            provider_model_id,
            format!("invalid Google response: {source}"),
        )
    })?;
    let candidate = parsed.candidates.into_iter().next().ok_or_else(|| {
        ModelError::provider(
            PROVIDER,
            provider_model_id,
            "Google response had no candidates",
        )
    })?;
    let content = candidate.content.unwrap_or_default();
    let mut assistant_parts = Vec::new();
    let mut tool_calls = Vec::new();

    for part in content.parts {
        if part.thought != Some(true)
            && let Some(text) = part.text
            && !text.trim().is_empty()
        {
            assistant_parts.push(text);
        }
        if let Some(function_call) = part.function_call {
            tool_calls.push(ToolCall {
                id: function_call
                    .id
                    .unwrap_or_else(|| format!("call_{}", tool_calls.len() + 1)),
                name: function_call.name,
                input: function_call.args.unwrap_or(Value::Null),
                operation_id: None,
            });
        }
    }

    let usage = parsed.usage_metadata.unwrap_or_default();
    let assistant_message = assistant_parts.join("\n");
    let finish_reason = finish_reason(candidate.finish_reason.as_deref(), !tool_calls.is_empty());
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
    let cached_input_tokens = u64::from(usage.cached_content_token_count);
    let input_tokens = u64::from(usage.prompt_token_count).saturating_sub(cached_input_tokens);
    let output_tokens = u64::from(usage.candidates_token_count);
    let reasoning_tokens = u64::from(usage.thoughts_token_count);
    let total_tokens = usage.total_token_count.map(u64::from).unwrap_or_else(|| {
        input_tokens
            .saturating_add(output_tokens)
            .saturating_add(cached_input_tokens)
            .saturating_add(reasoning_tokens)
    });
    Ok(ModelResponse {
        provider: PROVIDER.to_owned(),
        model_id: provider_model_id.to_owned(),
        catalog_model_id: Some(catalog_model_id.to_owned()),
        thinking_level: Some(thinking_level.as_str().to_owned()),
        assistant_message,
        finish_reason,
        tool_calls,
        structured_output,
        provider_context: Vec::new(),
        usage: ModelUsage {
            input_tokens,
            output_tokens,
            cached_input_tokens,
            reasoning_tokens,
            total_tokens,
            estimated_cost_microusd: 0,
            cost_lines: Vec::new(),
        },
    })
}

#[derive(Debug, Deserialize)]
struct GoogleResponse {
    #[serde(default)]
    candidates: Vec<GoogleCandidate>,
    #[serde(default, rename = "usageMetadata")]
    usage_metadata: Option<GoogleUsageMetadata>,
}

#[derive(Debug, Deserialize)]
struct GoogleCandidate {
    #[serde(default)]
    content: Option<GoogleContent>,
    #[serde(default, rename = "finishReason")]
    finish_reason: Option<String>,
}

#[derive(Debug, Default, Deserialize)]
struct GoogleContent {
    #[serde(default)]
    parts: Vec<GooglePart>,
}

#[derive(Debug, Deserialize)]
struct GooglePart {
    #[serde(default)]
    text: Option<String>,
    #[serde(default)]
    thought: Option<bool>,
    #[serde(default, rename = "functionCall")]
    function_call: Option<GoogleFunctionCall>,
}

#[derive(Debug, Deserialize)]
struct GoogleFunctionCall {
    #[serde(default)]
    id: Option<String>,
    name: String,
    #[serde(default)]
    args: Option<Value>,
}

#[derive(Clone, Debug, Default, Deserialize)]
struct GoogleUsageMetadata {
    #[serde(default, rename = "promptTokenCount")]
    prompt_token_count: u32,
    #[serde(default, rename = "candidatesTokenCount")]
    candidates_token_count: u32,
    #[serde(default, rename = "totalTokenCount")]
    total_token_count: Option<u32>,
    #[serde(default, rename = "cachedContentTokenCount")]
    cached_content_token_count: u32,
    #[serde(default, rename = "thoughtsTokenCount")]
    thoughts_token_count: u32,
}

fn finish_reason(value: Option<&str>, has_tool_calls: bool) -> FinishReason {
    match value {
        Some("STOP") if has_tool_calls => FinishReason::ToolCalls,
        Some("FINISH_REASON_UNSPECIFIED") if has_tool_calls => FinishReason::ToolCalls,
        None if has_tool_calls => FinishReason::ToolCalls,
        Some("STOP") => FinishReason::Stop,
        Some("MAX_TOKENS") => FinishReason::Truncated,
        Some(
            "SAFETY"
            | "BLOCKLIST"
            | "PROHIBITED_CONTENT"
            | "SPII"
            | "RECITATION"
            | "LANGUAGE"
            | "IMAGE_SAFETY"
            | "IMAGE_PROHIBITED_CONTENT"
            | "IMAGE_RECITATION",
        ) => FinishReason::Filtered,
        Some(raw) => FinishReason::Other(raw.to_owned()),
        None => FinishReason::Other("missing".to_owned()),
    }
}
