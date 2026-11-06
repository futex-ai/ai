//! Anthropic typed terminal metadata and validation.

use ai_interface::{
    Effort, ErrorKind, FinishReason, ModelEvent, ModelResult, ModelUsage, StructuredOutputSchema,
};
use ai_models_core::{ErrorContext, parse_structured_output};

use super::{replay::AnthropicAssistantBlock, stream::types::AnthropicCompletion};

const PROVIDER: &str = "anthropic";

pub(super) fn terminal_events(
    context: &ErrorContext,
    catalog_model_id: &str,
    provider_model_id: &str,
    thinking_level: Option<Effort>,
    completion: AnthropicCompletion,
    response_schema: Option<&StructuredOutputSchema>,
) -> ModelResult<[ModelEvent; 2]> {
    let finish_reason = finish_reason(completion.stop_reason.as_deref());
    if matches!(finish_reason, FinishReason::ToolCalls)
        && !completion
            .blocks
            .iter()
            .any(|block| matches!(block, AnthropicAssistantBlock::ToolUse { .. }))
    {
        return Err(context.model_error(
            ErrorKind::InvalidOutput,
            "tool_calls finish had no complete tool call",
        ));
    }
    let assistant_message = completion
        .blocks
        .iter()
        .filter_map(|block| match block {
            AnthropicAssistantBlock::Text { text } if !text.trim().is_empty() => {
                Some(text.as_str())
            }
            _ => None,
        })
        .collect::<Vec<_>>()
        .join("\n");
    let structured_output = match response_schema {
        Some(schema) if matches!(finish_reason, FinishReason::Stop) => Some(
            parse_structured_output(context, &assistant_message, schema)?,
        ),
        _ => None,
    };
    let input_tokens = u64::from(completion.usage.input_tokens)
        .saturating_add(u64::from(completion.usage.cache_creation_input_tokens));
    let output_tokens = u64::from(completion.usage.output_tokens);
    let cached_input_tokens = u64::from(completion.usage.cache_read_input_tokens);
    Ok([
        ModelEvent::Usage {
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
        },
        ModelEvent::Finish {
            finish_reason,
            provider: PROVIDER.to_owned(),
            model_id: provider_model_id.to_owned(),
            catalog_model_id: Some(catalog_model_id.to_owned()),
            thinking_level,
            structured_output,
        },
    ])
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
