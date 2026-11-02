//! Finish reasons and dispatchable tool-call validation.

use ai_interface::{FinishReason, ModelError, ModelResult, ToolCall};
use ai_models_core::parse_tool_call_arguments;
use serde_json::Value;

use super::{super::dialect::Dialect, types::ChatCompletionsToolCall};

pub(super) fn finish_reason(
    dialect: &Dialect,
    model_id: &str,
    value: Option<&str>,
) -> ModelResult<FinishReason> {
    Ok(match value {
        Some("stop") => FinishReason::Stop,
        Some("tool_calls") => FinishReason::ToolCalls,
        Some("length") => FinishReason::Truncated,
        Some("content_filter") => FinishReason::Filtered,
        Some("insufficient_system_resource") if dialect.required_reasoning => {
            return Err(ModelError::transient_provider(
                dialect.provider,
                model_id,
                "DeepSeek reported insufficient system resources",
            ));
        }
        Some(raw) => FinishReason::Other(raw.to_owned()),
        None => FinishReason::Other("missing".to_owned()),
    })
}

pub(super) fn dispatchable_tool_calls(
    dialect: &Dialect,
    model_id: &str,
    raw: Option<Value>,
    dispatchable: bool,
) -> ModelResult<Vec<ChatCompletionsToolCall>> {
    if !dispatchable {
        return Ok(Vec::new());
    }
    let missing = || {
        ModelError::provider(
            dialect.provider,
            model_id,
            format!("{} tool-call response had no tool calls", dialect.label),
        )
    };
    let Some(raw) = raw else {
        return Err(missing());
    };
    let calls: Vec<ChatCompletionsToolCall> = match serde_json::from_value(raw) {
        Ok(calls) => calls,
        Err(_) => {
            return Err(ModelError::provider(
                dialect.provider,
                model_id,
                format!("invalid {} tool-call payload", dialect.label),
            ));
        }
    };
    if calls.is_empty() {
        return Err(missing());
    }
    Ok(calls)
}

pub(super) fn parse_tool_calls(
    dialect: &Dialect,
    model_id: &str,
    calls: &[ChatCompletionsToolCall],
) -> ModelResult<Vec<ToolCall>> {
    calls
        .iter()
        .map(|call| {
            Ok(ToolCall {
                id: call.id.clone(),
                name: call.function.name.clone(),
                input: parse_tool_call_arguments(
                    dialect.provider,
                    model_id,
                    &call.function.arguments,
                )?,
                operation_id: None,
            })
        })
        .collect()
}
