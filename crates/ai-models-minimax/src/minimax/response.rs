//! MiniMax chat-completions response parsing.

use ai_interface::{
    FinishReason, MiniMaxReasoningDetail, ModelError, ModelResponse, ModelUsage,
    ProviderConversationItem, StructuredOutputSchema, ToolCall,
};
use ai_models_core::{
    ThinkingLevel, assistant_text, parse_structured_output, parse_tool_call_arguments,
};
use serde::Deserialize;
use serde_json::Value;

const PROVIDER: &str = "minimax";

pub(super) fn parse_response(
    catalog_model_id: &str,
    provider_model_id: &str,
    thinking_level: ThinkingLevel,
    body: Value,
    response_schema: Option<&StructuredOutputSchema>,
) -> std::result::Result<ModelResponse, ModelError> {
    let parsed: ChatCompletionsResponse = match serde_json::from_value(body) {
        Ok(parsed) => parsed,
        Err(source) => return Err(ModelError::internal(source)),
    };
    check_base_response(provider_model_id, parsed.base_resp.as_ref())?;
    let choice = parsed.choices.into_iter().next().ok_or_else(|| {
        ModelError::provider(
            PROVIDER,
            provider_model_id,
            "MiniMax response had no choices",
        )
    })?;
    let usage = parsed.usage.unwrap_or_default();
    let ChatCompletionsAssistantMessage {
        content,
        tool_calls,
        reasoning_content,
        reasoning_details,
    } = choice.message;
    let finish_reason = finish_reason(choice.finish_reason.as_deref());
    let has_tool_call_payload = !tool_calls.is_empty();
    let tool_calls = if finish_reason == FinishReason::ToolCalls {
        parse_tool_calls(provider_model_id, tool_calls)?
    } else {
        Vec::new()
    };
    let provider_context = provider_context(reasoning_content, reasoning_details);
    let assistant_message = assistant_text(content);
    let structured_output = response_schema
        .filter(|_| finish_reason == FinishReason::Stop && !has_tool_call_payload)
        .map(|schema| {
            parse_structured_output(PROVIDER, provider_model_id, &assistant_message, schema)
        })
        .transpose()?;

    Ok(ModelResponse {
        provider: PROVIDER.to_owned(),
        model_id: provider_model_id.to_owned(),
        catalog_model_id: Some(catalog_model_id.to_owned()),
        thinking_level: Some(thinking_level.as_str().to_owned()),
        assistant_message,
        tool_calls,
        finish_reason,
        structured_output,
        provider_context,
        usage: normalize_usage(usage),
    })
}

#[derive(Debug, Deserialize)]
struct ChatCompletionsResponse {
    #[serde(default)]
    choices: Vec<ChatCompletionsChoice>,
    #[serde(default)]
    usage: Option<ChatCompletionsUsage>,
    #[serde(default)]
    base_resp: Option<MiniMaxBaseResponse>,
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
    #[serde(default)]
    reasoning_content: Option<String>,
    #[serde(default)]
    reasoning_details: Vec<MiniMaxReasoningDetail>,
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
    prompt_tokens: u64,
    #[serde(default)]
    completion_tokens: u64,
    #[serde(default)]
    total_tokens: Option<u64>,
    #[serde(default)]
    prompt_tokens_details: PromptTokenDetails,
    #[serde(default)]
    completion_tokens_details: CompletionTokenDetails,
}

#[derive(Clone, Debug, Default, Deserialize)]
struct PromptTokenDetails {
    #[serde(default)]
    cached_tokens: u64,
}

#[derive(Clone, Debug, Default, Deserialize)]
struct CompletionTokenDetails {
    #[serde(default)]
    reasoning_tokens: u64,
}

#[derive(Debug, Deserialize)]
struct MiniMaxBaseResponse {
    #[serde(default)]
    status_code: Option<i64>,
    #[serde(default)]
    status_msg: Option<String>,
}

fn finish_reason(value: Option<&str>) -> FinishReason {
    match value {
        Some("stop") => FinishReason::Stop,
        Some("tool_calls") => FinishReason::ToolCalls,
        Some("length") => FinishReason::Truncated,
        Some("content_filter") => FinishReason::Filtered,
        Some(raw) => FinishReason::Other(raw.to_owned()),
        None => FinishReason::Other("missing".to_owned()),
    }
}

fn normalize_usage(usage: ChatCompletionsUsage) -> ModelUsage {
    let cached_input_tokens = usage.prompt_tokens_details.cached_tokens;
    let reasoning_tokens = usage.completion_tokens_details.reasoning_tokens;
    let input_tokens = usage.prompt_tokens.saturating_sub(cached_input_tokens);
    let output_tokens = usage.completion_tokens.saturating_sub(reasoning_tokens);
    let total_tokens = usage.total_tokens.unwrap_or_else(|| {
        input_tokens
            .saturating_add(output_tokens)
            .saturating_add(cached_input_tokens)
            .saturating_add(reasoning_tokens)
    });
    ModelUsage {
        input_tokens,
        output_tokens,
        cached_input_tokens,
        reasoning_tokens,
        total_tokens,
        estimated_cost_microusd: 0,
        cost_lines: Vec::new(),
    }
}

fn check_base_response(
    provider_model_id: &str,
    base_response: Option<&MiniMaxBaseResponse>,
) -> std::result::Result<(), ModelError> {
    match base_response_error(provider_model_id, base_response) {
        Some(error) => Err(error),
        None => Ok(()),
    }
}

pub(super) fn stream_base_response_error(
    provider_model_id: &str,
    body: &Value,
) -> std::result::Result<Option<ModelError>, serde_json::Error> {
    let envelope: MiniMaxBaseResponseEnvelope = serde_json::from_value(body.clone())?;
    Ok(base_response_error(
        provider_model_id,
        envelope.base_resp.as_ref(),
    ))
}

fn base_response_error(
    provider_model_id: &str,
    base_response: Option<&MiniMaxBaseResponse>,
) -> Option<ModelError> {
    let base_response = base_response?;
    let code = base_response.status_code.unwrap_or_default();
    if code == 0 {
        return None;
    }
    let status_message = base_response
        .status_msg
        .as_deref()
        .unwrap_or("unknown error");
    let message = format!("base_resp status_code {code}: {status_message}");
    let error = match code {
        1002 | 1041 | 2045 | 2056 => ModelError::rate_limited(PROVIDER, provider_model_id, message),
        1000 | 1001 | 1013 | 1024 | 1033 => {
            ModelError::transient_provider(PROVIDER, provider_model_id, message)
        }
        1039 => ModelError::context_limit_exceeded(PROVIDER, provider_model_id, message),
        _ => ModelError::provider(PROVIDER, provider_model_id, message),
    };
    Some(error)
}

#[derive(Debug, Deserialize)]
struct MiniMaxBaseResponseEnvelope {
    #[serde(default)]
    base_resp: Option<MiniMaxBaseResponse>,
}

fn parse_tool_calls(
    provider_model_id: &str,
    calls: Vec<ChatCompletionsToolCall>,
) -> std::result::Result<Vec<ToolCall>, ModelError> {
    calls
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
        .collect()
}

fn provider_context(
    reasoning_content: Option<String>,
    reasoning_details: Vec<MiniMaxReasoningDetail>,
) -> Vec<ProviderConversationItem> {
    if reasoning_content.is_none() && reasoning_details.is_empty() {
        return Vec::new();
    }
    vec![ProviderConversationItem::MiniMaxAssistant {
        reasoning_content,
        reasoning_details,
    }]
}
