//! Finish reasons and dispatchable tool-call validation.

use ai_interface::{ErrorKind, FinishReason, ModelError, ModelResult, ToolCall};
use ai_models_core::{ErrorContext, parse_tool_call_arguments};
use serde_json::Value;

use super::{
    super::dialect::{Dialect, ReplayRule},
    types::ChatCompletionsToolCall,
};

pub(super) fn finish_reason(
    context: &ErrorContext,
    dialect: &Dialect,
    value: Option<&str>,
) -> ModelResult<FinishReason> {
    Ok(match value {
        Some("stop") => FinishReason::Stop,
        Some("tool_calls") => FinishReason::ToolCalls,
        Some("function_call") if dialect.replay == ReplayRule::Xai => FinishReason::ToolCalls,
        Some("length") => FinishReason::Truncated,
        Some("content_filter") => FinishReason::Filtered,
        Some(raw)
            if dialect
                .transient_finish
                .is_some_and(|(reason, _)| reason == raw) =>
        {
            let message = dialect
                .transient_finish
                .map(|(_, message)| message)
                .unwrap_or_default();
            return Err(context.model_error(ErrorKind::Transient, message));
        }
        Some(raw) => FinishReason::Other(raw.to_owned()),
        None => FinishReason::Other("missing".to_owned()),
    })
}

pub(super) fn dispatchable_tool_calls(
    context: &ErrorContext,
    dialect: &Dialect,
    raw: Option<Value>,
    dispatchable: bool,
) -> ModelResult<Vec<ChatCompletionsToolCall>> {
    if !dispatchable {
        return Ok(Vec::new());
    }
    let missing = || {
        context.model_error(
            ErrorKind::InvalidOutput,
            format!("{} tool-call response had no tool calls", dialect.label),
        )
    };
    let Some(raw) = raw else {
        return Err(missing());
    };
    let calls: Vec<ChatCompletionsToolCall> = match serde_json::from_value(raw) {
        Ok(calls) => calls,
        Err(source) => {
            return Err(ModelError::with_source(
                context.core(
                    ErrorKind::InvalidOutput,
                    format!("invalid {} tool-call payload", dialect.label),
                ),
                source,
            ));
        }
    };
    if calls.is_empty() {
        return Err(missing());
    }
    Ok(calls)
}

pub(super) fn parse_tool_calls(
    context: &ErrorContext,
    dialect: &Dialect,
    calls: &[ChatCompletionsToolCall],
) -> ModelResult<Vec<ToolCall>> {
    calls
        .iter()
        .map(|call| {
            validate_identity(context, dialect, call)?;
            Ok(ToolCall {
                id: call.id.clone(),
                name: call.function.name.clone(),
                input: parse_tool_call_arguments(context, &call.function.arguments)?,
                operation_id: call.operation_id.clone(),
            })
        })
        .collect()
}

fn validate_identity(
    context: &ErrorContext,
    dialect: &Dialect,
    call: &ChatCompletionsToolCall,
) -> ModelResult<()> {
    if dialect.replay != ReplayRule::Qwen {
        return Ok(());
    }
    if call.id.trim().is_empty() {
        return Err(context.model_error(ErrorKind::InvalidOutput, "Qwen tool call had no id"));
    }
    if call.function.name.trim().is_empty() {
        return Err(context.model_error(
            ErrorKind::InvalidOutput,
            "Qwen tool call had no function name",
        ));
    }
    Ok(())
}
