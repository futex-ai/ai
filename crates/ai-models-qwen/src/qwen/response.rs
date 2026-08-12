//! QwenCloud Chat Completions response parsing.

use ai_interface::{
    FinishReason, ModelError, ModelResponse, ModelResult, ModelUsage, ProviderConversationItem,
    QwenToolCallContext, StructuredOutputSchema, ToolCall,
};
use ai_models_core::{
    ThinkingLevel, assistant_text, parse_structured_output, parse_tool_call_arguments,
};
use serde::Deserialize;
use serde_json::Value;

const PROVIDER: &str = "qwen";

pub(super) fn parse_response(
    catalog_model_id: &str,
    provider_model_id: &str,
    thinking_level: ThinkingLevel,
    body: Value,
    response_schema: Option<&StructuredOutputSchema>,
) -> ModelResult<ModelResponse> {
    let parsed: ChatCompletionsResponse = match serde_json::from_value(body) {
        Ok(parsed) => parsed,
        Err(source) => return Err(ModelError::internal(source)),
    };
    let ChatCompletionsResponse { choices, usage } = parsed;
    let Some(choice) = choices.into_iter().next() else {
        return Err(ModelError::provider(
            PROVIDER,
            provider_model_id,
            "Qwen response had no choices",
        ));
    };
    let finish_reason = finish_reason(choice.finish_reason.as_deref());
    let ChatCompletionsAssistantMessage {
        content,
        reasoning_content,
        tool_calls: raw_tool_call_payload,
    } = choice.message;
    let mut assistant_message = assistant_text(content.clone());
    let dispatchable_calls = matches!(finish_reason, FinishReason::ToolCalls);
    if dispatchable_calls && thinking_level.is_enabled() && reasoning_content.is_none() {
        return Err(ModelError::provider(
            PROVIDER,
            provider_model_id,
            "Qwen thinking tool response had no reasoning content",
        ));
    }
    let parsed_tool_calls =
        dispatchable_tool_calls(provider_model_id, raw_tool_call_payload, dispatchable_calls)?;
    let tool_calls = parse_tool_calls(provider_model_id, &parsed_tool_calls)?;
    let structured_output = response_schema
        .filter(|_| matches!(finish_reason, FinishReason::Stop) && tool_calls.is_empty())
        .map(|schema| {
            parse_structured_output(PROVIDER, provider_model_id, &assistant_message, schema)
        })
        .transpose()?;
    if assistant_message.trim().is_empty()
        && let Some(output) = structured_output.as_ref()
    {
        assistant_message = output.to_string();
    }
    Ok(ModelResponse {
        provider: PROVIDER.to_owned(),
        model_id: provider_model_id.to_owned(),
        catalog_model_id: Some(catalog_model_id.to_owned()),
        thinking_level: Some(thinking_level.as_str().to_owned()),
        assistant_message,
        tool_calls,
        finish_reason,
        structured_output,
        provider_context: vec![ProviderConversationItem::QwenAssistantMessage {
            content,
            reasoning_content,
            tool_calls: raw_tool_calls(&parsed_tool_calls),
        }],
        usage: normalize_usage(usage.unwrap_or_default()),
    })
}

#[derive(Debug, Deserialize)]
struct ChatCompletionsResponse {
    #[serde(default)]
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
    reasoning_content: Option<String>,
    #[serde(default)]
    tool_calls: Option<Value>,
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
    prompt_tokens_details: ChatCompletionsPromptTokenDetails,
}

#[derive(Clone, Debug, Default, Deserialize)]
struct ChatCompletionsPromptTokenDetails {
    #[serde(default)]
    cached_tokens: u64,
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

fn dispatchable_tool_calls(
    model_id: &str,
    raw_tool_calls: Option<Value>,
    dispatchable: bool,
) -> ModelResult<Vec<ChatCompletionsToolCall>> {
    if !dispatchable {
        return Ok(Vec::new());
    }
    let Some(raw_tool_calls) = raw_tool_calls else {
        return Err(missing_tool_calls(model_id));
    };
    let tool_calls: Vec<ChatCompletionsToolCall> = match serde_json::from_value(raw_tool_calls) {
        Ok(tool_calls) => tool_calls,
        Err(_) => {
            return Err(ModelError::provider(
                PROVIDER,
                model_id,
                "invalid Qwen tool-call payload",
            ));
        }
    };
    if tool_calls.is_empty() {
        return Err(missing_tool_calls(model_id));
    }
    Ok(tool_calls)
}

fn missing_tool_calls(model_id: &str) -> ModelError {
    ModelError::provider(
        PROVIDER,
        model_id,
        "Qwen tool-call response had no tool calls",
    )
}

fn parse_tool_calls(
    model_id: &str,
    calls: &[ChatCompletionsToolCall],
) -> ModelResult<Vec<ToolCall>> {
    calls
        .iter()
        .map(|call| {
            validate_tool_call_identity(model_id, call)?;
            Ok(ToolCall {
                id: call.id.clone(),
                name: call.function.name.clone(),
                input: parse_tool_call_arguments(PROVIDER, model_id, &call.function.arguments)?,
                operation_id: None,
            })
        })
        .collect()
}

fn validate_tool_call_identity(model_id: &str, call: &ChatCompletionsToolCall) -> ModelResult<()> {
    if call.id.trim().is_empty() {
        return Err(ModelError::provider(
            PROVIDER,
            model_id,
            "Qwen tool call had no id",
        ));
    }
    if call.function.name.trim().is_empty() {
        return Err(ModelError::provider(
            PROVIDER,
            model_id,
            "Qwen tool call had no function name",
        ));
    }
    Ok(())
}

fn raw_tool_calls(calls: &[ChatCompletionsToolCall]) -> Vec<QwenToolCallContext> {
    calls
        .iter()
        .map(|call| QwenToolCallContext {
            id: call.id.clone(),
            name: call.function.name.clone(),
            arguments: call.function.arguments.clone(),
        })
        .collect()
}

fn normalize_usage(usage: ChatCompletionsUsage) -> ModelUsage {
    let cached_input_tokens = usage.prompt_tokens_details.cached_tokens;
    let input_tokens = usage.prompt_tokens.saturating_sub(cached_input_tokens);
    let output_tokens = usage.completion_tokens;
    let total_tokens = usage.total_tokens.unwrap_or_else(|| {
        input_tokens
            .saturating_add(cached_input_tokens)
            .saturating_add(output_tokens)
    });
    ModelUsage {
        input_tokens,
        output_tokens,
        cached_input_tokens,
        cache_write_input_tokens: 0,
        reasoning_tokens: 0,
        total_tokens,
        estimated_cost_microusd: 0,
        cost_lines: Vec::new(),
    }
}
