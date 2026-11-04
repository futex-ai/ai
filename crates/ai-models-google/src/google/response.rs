//! Google typed terminal metadata and validation.

use ai_interface::{
    Effort, FinishReason, ModelEvent, ModelResult, ModelUsage, StructuredOutputSchema,
};
use ai_models_core::{ErrorContext, parse_structured_output};

use super::stream::types::{GoogleCompletion, GoogleUsageMetadata};

const PROVIDER: &str = "google";

pub(super) fn terminal_events(
    context: &ErrorContext,
    catalog_model_id: &str,
    provider_model_id: &str,
    thinking_level: Option<Effort>,
    completion: GoogleCompletion,
    response_schema: Option<&StructuredOutputSchema>,
) -> ModelResult<Vec<ModelEvent>> {
    let finish_reason = if completion.candidate_seen {
        finish_reason(
            completion.finish_reason.as_deref(),
            completion.tool_call_count > 0,
        )
    } else {
        FinishReason::Filtered
    };
    let structured_output = match response_schema {
        Some(schema) if matches!(finish_reason, FinishReason::Stop) => Some(
            parse_structured_output(context, &completion.assistant_message, schema)?,
        ),
        _ => None,
    };
    let mut events = completion.state_events;
    events.extend([
        ModelEvent::Usage {
            usage: model_usage(completion.usage),
        },
        ModelEvent::Finish {
            finish_reason,
            provider: PROVIDER.to_owned(),
            model_id: provider_model_id.to_owned(),
            catalog_model_id: Some(catalog_model_id.to_owned()),
            thinking_level,
            structured_output,
        },
    ]);
    Ok(events)
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

fn model_usage(usage: GoogleUsageMetadata) -> ModelUsage {
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
