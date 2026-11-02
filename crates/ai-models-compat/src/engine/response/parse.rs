//! Shared response normalization and local structured-output checks.

use ai_interface::{FinishReason, ModelError, ModelResponse, ModelResult, StructuredOutputSchema};
use ai_models_core::{assistant_text, parse_structured_output};
use serde_json::Value;

use super::{
    super::dialect::{CompletionContext, ReplayRule},
    eager, legacy,
    mapping::{dispatchable_tool_calls, finish_reason, parse_tool_calls},
    replay::provider_context,
    types::{ChatCompletionsAssistantMessage, ChatCompletionsResponse},
    usage::normalize,
};

pub(crate) fn parse_response(
    context: CompletionContext<'_>,
    body: Value,
    response_schema: Option<&StructuredOutputSchema>,
) -> ModelResult<ModelResponse> {
    let CompletionContext {
        dialect,
        catalog_model_id,
        provider_model_id,
        thinking_level,
        synthetic_tool_call_scope: _,
    } = context;
    let parse_error = |source| match dialect.replay {
        ReplayRule::Kimi => ModelError::provider(
            dialect.provider,
            provider_model_id,
            "invalid Kimi response payload",
        ),
        ReplayRule::Xai => ModelError::provider(
            dialect.provider,
            provider_model_id,
            format!("invalid xAI response: {source}"),
        ),
        _ => ModelError::internal(source),
    };
    let mut body = body;
    if dialect.replay == ReplayRule::Kimi && body.get("choices").is_none() {
        return Err(ModelError::provider(
            dialect.provider,
            provider_model_id,
            "invalid Kimi response payload",
        ));
    }
    let parsed: ChatCompletionsResponse = match match dialect.replay {
        ReplayRule::MiniMax => eager::decode(body.take()),
        ReplayRule::Xai => legacy::decode(body.take()),
        _ => serde_json::from_value(body.take()),
    } {
        Ok(parsed) => parsed,
        Err(source) => return Err(parse_error(source)),
    };
    let usage = match normalize(dialect.replay, parsed.usage) {
        Ok(usage) => usage,
        Err(source) => return Err(parse_error(source)),
    };
    if let Some(error) = dialect
        .base_error
        .and_then(|classify| classify(provider_model_id, parsed.base_resp.as_ref()))
    {
        return Err(error);
    }
    let Some(choice) = parsed.choices.into_iter().next() else {
        return Err(ModelError::provider(
            dialect.provider,
            provider_model_id,
            format!("{} response had no choices", dialect.label),
        ));
    };
    let finish_reason = finish_reason(dialect, provider_model_id, choice.finish_reason.as_deref())?;
    let ChatCompletionsAssistantMessage {
        content,
        reasoning_content,
        tool_calls: raw,
        parsed_tool_calls: mut eager_calls,
        reasoning_details,
        function_call,
    } = choice.message;
    let mut assistant_message = assistant_text(content.clone());
    let dispatchable = matches!(finish_reason, FinishReason::ToolCalls);
    if dispatchable
        && dialect.required_reasoning
        && thinking_level.is_enabled()
        && reasoning_content.is_none()
    {
        return Err(ModelError::provider(
            dialect.provider,
            provider_model_id,
            format!(
                "{} thinking tool response had no reasoning content",
                dialect.label
            ),
        ));
    }
    let legacy_context = legacy::normalize(context, &mut eager_calls, function_call);
    let has_raw_calls = eager_calls.as_ref().is_some_and(|calls| !calls.is_empty());
    let parsed_calls = match eager_calls {
        Some(calls) if dispatchable => calls,
        Some(_) => Vec::new(),
        None => dispatchable_tool_calls(dialect, provider_model_id, raw, dispatchable)?,
    };
    let tool_calls = parse_tool_calls(dialect, provider_model_id, &parsed_calls)?;
    let structured_output = response_schema
        .filter(|_| {
            matches!(finish_reason, FinishReason::Stop)
                && tool_calls.is_empty()
                && !(dialect.replay == ReplayRule::MiniMax && has_raw_calls)
        })
        .map(|schema| {
            parse_structured_output(
                dialect.provider,
                provider_model_id,
                &assistant_message,
                schema,
            )
        })
        .transpose()?;
    if matches!(dialect.replay, ReplayRule::Kimi | ReplayRule::Xai)
        && assistant_message.trim().is_empty()
    {
        assistant_message = structured_output
            .as_ref()
            .map(Value::to_string)
            .unwrap_or_default();
    }
    if dialect.replay == ReplayRule::Qwen
        && assistant_message.trim().is_empty()
        && let Some(output) = structured_output.as_ref()
    {
        assistant_message = output.to_string();
    }
    Ok(ModelResponse {
        provider: dialect.provider.to_owned(),
        model_id: provider_model_id.to_owned(),
        catalog_model_id: Some(catalog_model_id.to_owned()),
        thinking_level: Some(thinking_level.as_str().to_owned()),
        assistant_message,
        tool_calls,
        finish_reason,
        structured_output,
        provider_context: if dialect.replay == ReplayRule::Xai {
            legacy_context
        } else {
            provider_context(
                dialect.replay,
                dispatchable,
                content,
                reasoning_content,
                &parsed_calls,
                reasoning_details,
            )
        },
        usage,
    })
}
