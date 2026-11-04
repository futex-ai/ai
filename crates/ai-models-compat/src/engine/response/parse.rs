//! Shared response normalization and local structured-output checks.

use ai_interface::{
    Effort, ErrorKind, FinishReason, ModelError, ModelResponse, ModelResult, ModelUsage,
    StructuredOutputSchema,
};
use ai_models_core::{assistant_text, parse_structured_output};
use serde_json::Value;

use super::{
    super::dialect::{CompletionContext, ReplayRule},
    eager, legacy,
    mapping::{dispatchable_tool_calls, finish_reason, parse_tool_calls},
    replay::{ResponseContent, parts},
    types::{ChatCompletionsAssistantMessage, ChatCompletionsResponse},
    usage::normalize,
};

#[cfg(test)]
pub(crate) fn parse_response(
    context: CompletionContext<'_>,
    body: Value,
    response_schema: Option<&StructuredOutputSchema>,
) -> ModelResult<ModelResponse> {
    let (parsed, usage) = decode_response(context, body)?;
    map_response(context, parsed, usage, response_schema)
}

pub(super) fn decode_response(
    context: CompletionContext<'_>,
    body: Value,
) -> ModelResult<(ChatCompletionsResponse, ModelUsage)> {
    let CompletionContext {
        dialect,
        error,
        catalog_model_id: _,
        provider_model_id: _,
        thinking_level: _,
        synthetic_tool_call_scope: _,
    } = context;
    let parse_error = |source| match dialect.replay {
        ReplayRule::Kimi => ModelError::with_source(
            error.core(ErrorKind::InvalidOutput, "invalid Kimi response payload"),
            error.source(source),
        ),
        ReplayRule::Xai => ModelError::with_source(
            error.core(
                ErrorKind::InvalidOutput,
                format!("invalid xAI response: {source}"),
            ),
            error.source(source),
        ),
        _ => ModelError::with_source(
            error.core(ErrorKind::InvalidOutput, "invalid provider response"),
            error.source(source),
        ),
    };
    let mut body = body;
    if dialect.replay == ReplayRule::Kimi && body.get("choices").is_none() {
        return Err(error.model_error(ErrorKind::InvalidOutput, "invalid Kimi response payload"));
    }
    let mut parsed: ChatCompletionsResponse = match match dialect.replay {
        ReplayRule::MiniMax => eager::decode(body.take()),
        ReplayRule::Xai => legacy::decode(body.take()),
        _ => serde_json::from_value(body.take()),
    } {
        Ok(parsed) => parsed,
        Err(source) => return Err(parse_error(source)),
    };
    let usage = match normalize(dialect.usage, parsed.usage.take()) {
        Ok(usage) => usage,
        Err(source) => return Err(parse_error(source)),
    };
    Ok((parsed, usage))
}

pub(super) fn map_response(
    context: CompletionContext<'_>,
    parsed: ChatCompletionsResponse,
    usage: ModelUsage,
    response_schema: Option<&StructuredOutputSchema>,
) -> ModelResult<ModelResponse> {
    let CompletionContext {
        dialect,
        error,
        catalog_model_id,
        provider_model_id,
        thinking_level,
        synthetic_tool_call_scope: _,
    } = context;
    if let Some(error) = dialect
        .base_error
        .and_then(|classify| classify(error, parsed.base_resp.as_ref()))
    {
        return Err(error);
    }
    let Some(choice) = parsed.choices.into_iter().next() else {
        return Err(error.model_error(
            ErrorKind::InvalidOutput,
            format!("{} response had no choices", dialect.label),
        ));
    };
    let finish_reason = finish_reason(error, dialect, choice.finish_reason.as_deref())?;
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
        && thinking_level.is_none_or(Effort::is_enabled)
        && reasoning_content.is_none()
    {
        return Err(error.model_error(
            ErrorKind::InvalidOutput,
            format!(
                "{} thinking tool response had no reasoning content",
                dialect.label
            ),
        ));
    }
    let legacy_context = legacy::normalize(context, &mut eager_calls, function_call);
    let parsed_calls = match eager_calls {
        Some(calls) if dispatchable => calls,
        Some(_) => Vec::new(),
        None => dispatchable_tool_calls(error, dialect, raw, dispatchable)?,
    };
    if dispatchable && parsed_calls.is_empty() {
        return Err(error.model_error(
            ErrorKind::InvalidOutput,
            format!("{} tool-call response had no tool calls", dialect.label),
        ));
    }
    let tool_calls = parse_tool_calls(error, dialect, &parsed_calls)?;
    let structured_output = response_schema
        .filter(|_| matches!(finish_reason, FinishReason::Stop) && tool_calls.is_empty())
        .map(|schema| parse_structured_output(error, &assistant_message, schema))
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
        thinking_level,
        parts: parts(
            context,
            ResponseContent {
                text: assistant_message,
                raw_content: content,
                reasoning: reasoning_content,
                details: reasoning_details,
                calls: tool_calls,
                raw_calls: parsed_calls,
                legacy: legacy_context,
                dispatchable,
            },
        )?,
        finish_reason,
        structured_output,
        usage,
        attempts: Vec::new(),
    })
}
