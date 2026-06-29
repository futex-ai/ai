//! OpenAI Responses response parsing.

mod finish;
mod output;
mod types;

use ai_interface::{FinishReason, ModelError, ModelResponse, ModelUsage, StructuredOutputSchema};
use ai_models_core::{ThinkingLevel, parse_structured_output};
use serde_json::Value;

use self::finish::finish_reason;
use self::output::{assistant_message, has_function_calls, provider_context, tool_calls};
use self::types::{ResponsesError, ResponsesResponse};

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
    let provider_context = provider_context(&parsed.output);
    let finish_reason = finish_reason(&parsed, has_function_calls(&parsed.output));
    let tool_calls = if matches!(finish_reason, FinishReason::ToolCalls) {
        tool_calls(provider_model_id, &parsed.output)?
    } else {
        Vec::new()
    };
    let usage = parsed.usage.unwrap_or_default();
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

fn response_error_message(error: ResponsesError) -> String {
    error
        .message
        .unwrap_or_else(|| "OpenAI Responses response contained an error".to_owned())
}
