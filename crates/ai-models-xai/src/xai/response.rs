//! xAI chat-completions response parsing.

use ai_interface::{
    FinishReason, ModelError, ModelResponse, ModelUsage, StructuredOutputSchema, ToolCall,
};
use ai_models_core::{
    ThinkingLevel, assistant_text, parse_structured_output, parse_tool_call_arguments,
};
use serde::Deserialize;
use serde_json::Value;

const PROVIDER: &str = "xai";

pub(super) fn parse_response(
    catalog_model_id: &str,
    provider_model_id: &str,
    thinking_level: ThinkingLevel,
    body: Value,
    response_schema: Option<&StructuredOutputSchema>,
) -> std::result::Result<ModelResponse, ModelError> {
    let parsed: ChatCompletionsResponse = serde_json::from_value(body).map_err(|source| {
        ModelError::provider(
            PROVIDER,
            provider_model_id,
            format!("invalid xAI response: {source}"),
        )
    })?;
    let choice = parsed.choices.into_iter().next().ok_or_else(|| {
        ModelError::provider(PROVIDER, provider_model_id, "xAI response had no choices")
    })?;
    let tool_calls = choice
        .message
        .tool_calls
        .into_iter()
        .map(|call| {
            Ok(ToolCall {
                id: call.id,
                name: call.function.name,
                input: parse_tool_call_arguments(
                    PROVIDER,
                    provider_model_id,
                    &call.function.arguments,
                )?,
                operation_id: None,
            })
        })
        .collect::<std::result::Result<Vec<_>, ModelError>>()?;
    let usage = parsed.usage.unwrap_or_default();
    let assistant_message = assistant_text(choice.message.content);
    let structured_output = response_schema
        .filter(|_| tool_calls.is_empty())
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
    let cached_input_tokens = u64::from(usage.prompt_tokens_details.cached_tokens);
    let reasoning_tokens = u64::from(usage.completion_tokens_details.reasoning_tokens);
    let input_tokens = u64::from(usage.prompt_tokens).saturating_sub(cached_input_tokens);
    let output_tokens = u64::from(usage.completion_tokens).saturating_sub(reasoning_tokens);
    let total_tokens = usage.total_tokens.map(u64::from).unwrap_or_else(|| {
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
        tool_calls,
        finish_reason: finish_reason(choice.finish_reason.as_deref()),
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
struct ChatCompletionsResponse {
    choices: Vec<ChatCompletionsChoice>,
    #[serde(default)]
    usage: Option<ChatCompletionsUsage>,
}

#[derive(Debug, Deserialize)]
struct ChatCompletionsChoice {
    message: ChatCompletionsAssistantMessage,
    #[serde(default)]
    finish_reason: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ChatCompletionsAssistantMessage {
    #[serde(default)]
    content: Option<String>,
    #[serde(default)]
    tool_calls: Vec<ChatCompletionsToolCall>,
}

#[derive(Debug, Deserialize)]
struct ChatCompletionsToolCall {
    id: String,
    function: ChatCompletionsToolFunction,
}

#[derive(Debug, Deserialize)]
struct ChatCompletionsToolFunction {
    name: String,
    arguments: String,
}

#[derive(Clone, Debug, Default, Deserialize)]
struct ChatCompletionsUsage {
    #[serde(default)]
    prompt_tokens: u32,
    #[serde(default)]
    completion_tokens: u32,
    #[serde(default)]
    total_tokens: Option<u32>,
    #[serde(default)]
    prompt_tokens_details: ChatCompletionsPromptTokenDetails,
    #[serde(default)]
    completion_tokens_details: ChatCompletionsCompletionTokenDetails,
}

#[derive(Clone, Debug, Default, Deserialize)]
struct ChatCompletionsPromptTokenDetails {
    #[serde(default)]
    cached_tokens: u32,
}

#[derive(Clone, Debug, Default, Deserialize)]
struct ChatCompletionsCompletionTokenDetails {
    #[serde(default)]
    reasoning_tokens: u32,
}

fn finish_reason(value: Option<&str>) -> FinishReason {
    match value {
        Some("stop") => FinishReason::Stop,
        Some("tool_calls" | "function_call") => FinishReason::ToolCalls,
        Some("length") => FinishReason::Truncated,
        Some("content_filter") => FinishReason::Filtered,
        Some(raw) => FinishReason::Other(raw.to_owned()),
        None => FinishReason::Other("missing".to_owned()),
    }
}
