//! OpenAI Responses response parsing.

use ai_interface::{
    FinishReason, ModelError, ModelResponse, ModelUsage, OpenAiReasoningSummary,
    ProviderConversationItem, StructuredOutputSchema, ToolCall,
};
use ai_models_core::{ThinkingLevel, parse_structured_output, parse_tool_call_arguments};
use serde::Deserialize;
use serde_json::Value;

const PROVIDER: &str = "openai";

pub(super) fn parse_response(
    catalog_model_id: &str,
    provider_model_id: &str,
    thinking_level: ThinkingLevel,
    body: Value,
    response_schema: Option<&StructuredOutputSchema>,
) -> std::result::Result<ModelResponse, ModelError> {
    let parsed: ResponsesResponse = match serde_json::from_value(body) {
        Ok(parsed) => parsed,
        Err(source) => {
            return Err(ModelError::provider(
                PROVIDER,
                provider_model_id,
                format!("invalid OpenAI Responses response: {source}"),
            ));
        }
    };
    if let Some(error) = parsed.error {
        return Err(ModelError::provider(
            PROVIDER,
            provider_model_id,
            response_error_message(error),
        ));
    }

    let assistant_message = assistant_message(&parsed.output);
    let tool_calls = tool_calls(provider_model_id, &parsed.output)?;
    let provider_context = provider_context(&parsed.output);
    let finish_reason = finish_reason(&parsed, !tool_calls.is_empty());
    let usage = parsed.usage.unwrap_or_default();
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

    let cached_input_tokens = usage.input_tokens_details.cached_tokens;
    let reasoning_tokens = usage.output_tokens_details.reasoning_tokens;
    let input_tokens = usage.input_tokens.saturating_sub(cached_input_tokens);
    let output_tokens = usage.output_tokens.saturating_sub(reasoning_tokens);
    let total_tokens = usage.total_tokens.unwrap_or_else(|| {
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
        finish_reason,
        structured_output,
        provider_context,
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
struct ResponsesResponse {
    #[serde(default)]
    status: Option<String>,
    #[serde(default)]
    output: Vec<ResponsesOutputItem>,
    #[serde(default)]
    usage: Option<ResponsesUsage>,
    #[serde(default)]
    incomplete_details: Option<ResponsesIncompleteDetails>,
    #[serde(default)]
    error: Option<ResponsesError>,
}

#[derive(Debug, Deserialize)]
#[serde(tag = "type")]
enum ResponsesOutputItem {
    #[serde(rename = "message")]
    Message { content: Vec<ResponsesContentPart> },
    #[serde(rename = "reasoning")]
    Reasoning {
        id: String,
        #[serde(default)]
        summary: Vec<OpenAiReasoningSummary>,
        #[serde(default)]
        encrypted_content: Option<String>,
    },
    #[serde(rename = "function_call")]
    FunctionCall {
        call_id: String,
        name: String,
        arguments: String,
    },
    #[serde(other)]
    Other,
}

#[derive(Debug, Deserialize)]
#[serde(tag = "type")]
enum ResponsesContentPart {
    #[serde(rename = "output_text")]
    OutputText { text: String },
    #[serde(rename = "refusal")]
    Refusal { refusal: String },
    #[serde(other)]
    Other,
}

#[derive(Clone, Debug, Default, Deserialize)]
struct ResponsesUsage {
    #[serde(default)]
    input_tokens: u64,
    #[serde(default)]
    output_tokens: u64,
    #[serde(default)]
    total_tokens: Option<u64>,
    #[serde(default)]
    input_tokens_details: ResponsesInputTokenDetails,
    #[serde(default)]
    output_tokens_details: ResponsesOutputTokenDetails,
}

#[derive(Clone, Debug, Default, Deserialize)]
struct ResponsesInputTokenDetails {
    #[serde(default)]
    cached_tokens: u64,
}

#[derive(Clone, Debug, Default, Deserialize)]
struct ResponsesOutputTokenDetails {
    #[serde(default)]
    reasoning_tokens: u64,
}

#[derive(Debug, Deserialize)]
struct ResponsesIncompleteDetails {
    reason: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ResponsesError {
    message: Option<String>,
}

fn assistant_message(output: &[ResponsesOutputItem]) -> String {
    output
        .iter()
        .flat_map(|item| match item {
            ResponsesOutputItem::Message { content } => content
                .iter()
                .filter_map(|part| match part {
                    ResponsesContentPart::OutputText { text } => Some(text.clone()),
                    ResponsesContentPart::Refusal { refusal } => Some(refusal.clone()),
                    ResponsesContentPart::Other => None,
                })
                .collect::<Vec<_>>(),
            ResponsesOutputItem::Reasoning { .. }
            | ResponsesOutputItem::FunctionCall { .. }
            | ResponsesOutputItem::Other => Vec::new(),
        })
        .collect::<Vec<_>>()
        .join("")
}

fn provider_context(output: &[ResponsesOutputItem]) -> Vec<ProviderConversationItem> {
    output
        .iter()
        .filter_map(|item| match item {
            ResponsesOutputItem::Reasoning {
                id,
                summary,
                encrypted_content,
            } => Some(ProviderConversationItem::OpenAiReasoning {
                id: id.clone(),
                summary: summary.clone(),
                encrypted_content: encrypted_content.clone(),
            }),
            ResponsesOutputItem::Message { .. }
            | ResponsesOutputItem::FunctionCall { .. }
            | ResponsesOutputItem::Other => None,
        })
        .collect()
}

fn tool_calls(
    provider_model_id: &str,
    output: &[ResponsesOutputItem],
) -> std::result::Result<Vec<ToolCall>, ModelError> {
    output
        .iter()
        .filter_map(|item| match item {
            ResponsesOutputItem::FunctionCall {
                call_id,
                name,
                arguments,
            } => Some((call_id, name, arguments)),
            ResponsesOutputItem::Message { .. }
            | ResponsesOutputItem::Reasoning { .. }
            | ResponsesOutputItem::Other => None,
        })
        .map(|(call_id, name, arguments)| {
            Ok(ToolCall {
                id: call_id.clone(),
                name: name.clone(),
                input: parse_tool_call_arguments(PROVIDER, provider_model_id, arguments)?,
                operation_id: None,
            })
        })
        .collect()
}

fn finish_reason(response: &ResponsesResponse, has_tool_calls: bool) -> FinishReason {
    if has_tool_calls {
        return FinishReason::ToolCalls;
    }
    if has_refusal(&response.output) {
        return FinishReason::Filtered;
    }
    match response.status.as_deref() {
        Some("completed") | None => FinishReason::Stop,
        Some("incomplete") => incomplete_finish_reason(response),
        Some("failed" | "cancelled") => FinishReason::Other(
            response
                .status
                .clone()
                .unwrap_or_else(|| "failed".to_owned()),
        ),
        Some(raw) => FinishReason::Other(raw.to_owned()),
    }
}

fn incomplete_finish_reason(response: &ResponsesResponse) -> FinishReason {
    match response
        .incomplete_details
        .as_ref()
        .and_then(|details| details.reason.as_deref())
    {
        Some("max_output_tokens") => FinishReason::Truncated,
        Some("content_filter") => FinishReason::Filtered,
        Some(raw) => FinishReason::Other(raw.to_owned()),
        None => FinishReason::Other("incomplete".to_owned()),
    }
}

fn has_refusal(output: &[ResponsesOutputItem]) -> bool {
    output.iter().any(|item| match item {
        ResponsesOutputItem::Message { content } => content
            .iter()
            .any(|part| matches!(part, ResponsesContentPart::Refusal { .. })),
        ResponsesOutputItem::Reasoning { .. }
        | ResponsesOutputItem::FunctionCall { .. }
        | ResponsesOutputItem::Other => false,
    })
}

fn response_error_message(error: ResponsesError) -> String {
    error
        .message
        .unwrap_or_else(|| "OpenAI Responses response contained an error".to_owned())
}
