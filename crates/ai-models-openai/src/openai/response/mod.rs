//! OpenAI typed terminal metadata and output checks.

mod finish;
mod output;
pub(super) mod types;

use ai_interface::{
    Effort, FinishReason, ModelError, ModelEvent, ModelResult, ModelUsage, StructuredOutputSchema,
};
use ai_models_core::parse_structured_output;

use self::types::{ResponsesError, ResponsesResponse};

const PROVIDER: &str = "openai";

pub(super) fn check_error(model_id: &str, parsed: &ResponsesResponse) -> ModelResult<()> {
    if let Some(error) = &parsed.error {
        return Err(ModelError::provider(
            PROVIDER,
            model_id,
            response_error_message(error),
        ));
    }
    Ok(())
}

pub(super) fn finish_reason(parsed: &ResponsesResponse) -> FinishReason {
    finish::finish_reason(parsed, output::has_function_calls(&parsed.output))
}

pub(super) fn terminal_events(
    catalog_model_id: &str,
    provider_model_id: &str,
    thinking_level: Effort,
    parsed: ResponsesResponse,
    assistant_message: &str,
    response_schema: Option<&StructuredOutputSchema>,
) -> ModelResult<[ModelEvent; 2]> {
    let finish_reason = finish_reason(&parsed);
    let provider_context = output::provider_context(
        &parsed.output,
        matches!(finish_reason, FinishReason::ToolCalls),
    );
    let structured_output = match response_schema {
        Some(schema) if matches!(finish_reason, FinishReason::Stop) => Some(
            parse_structured_output(PROVIDER, provider_model_id, assistant_message, schema)?,
        ),
        _ => None,
    };
    let usage = parsed.usage.unwrap_or_default();
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
    Ok([
        ModelEvent::Usage {
            usage: ModelUsage {
                input_tokens,
                output_tokens,
                cached_input_tokens,
                reasoning_tokens,
                total_tokens,
                estimated_cost_microusd: 0,
                cost_lines: Vec::new(),
            },
        },
        ModelEvent::Finish {
            finish_reason,
            provider: PROVIDER.to_owned(),
            model_id: provider_model_id.to_owned(),
            catalog_model_id: Some(catalog_model_id.to_owned()),
            thinking_level: Some(thinking_level),
            structured_output,
            provider_context,
        },
    ])
}

fn response_error_message(error: &ResponsesError) -> String {
    error
        .message
        .clone()
        .unwrap_or_else(|| "OpenAI Responses response contained an error".to_owned())
}
