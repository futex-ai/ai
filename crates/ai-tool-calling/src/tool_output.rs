//! Runtime integration for model-visible tool output envelopes.

use ai_interface::{ToolOutputEnvelope, ToolOutputId, ToolOutputRemainderUnavailableReason};
use serde_json::Value;

use crate::{
    Error, Result, ToolCallingRuntime, ToolOutputStoreError, ToolOutputStoreWindow,
    ToolOutputWriteRequest,
};

pub(crate) const TOOL_OUTPUT_READ: &str = "tool_output_read";

/// Successful output representations prepared for runtime records and logs.
pub(crate) struct ManagedToolOutput {
    /// Output id present for stored outputs.
    pub(crate) output_id: Option<ToolOutputId>,
    /// Raw output returned by the tool implementation.
    pub(crate) raw_output: Value,
    /// Bounded model-visible envelope.
    pub(crate) model_visible_output: ToolOutputEnvelope,
}

pub(crate) async fn manage_successful_tool_output(
    runtime: &ToolCallingRuntime,
    tool_name: &str,
    raw_output: Value,
) -> Result<ManagedToolOutput> {
    let serialized = match serde_json::to_string(&raw_output) {
        Ok(serialized) => serialized,
        Err(source) => {
            return Err(Error::OutputSerialization {
                tool_name: tool_name.to_owned(),
                source,
            });
        }
    };
    let policy = runtime.output_policy();
    let total_bytes = serialized.len();
    if total_bytes <= policy.inline_limit_bytes() {
        return Ok(ManagedToolOutput {
            output_id: None,
            raw_output: raw_output.clone(),
            model_visible_output: ToolOutputEnvelope::inline(
                tool_name.to_owned(),
                raw_output,
                total_bytes,
            ),
        });
    }

    let write = runtime
        .output_store()
        .write(ToolOutputWriteRequest {
            tool_name: tool_name.to_owned(),
            content: serialized,
            policy,
            first_window_length: policy.inline_limit_bytes(),
        })
        .await;
    match write {
        Ok(result) => {
            let output_id = result.output_id.clone();
            Ok(ManagedToolOutput {
                output_id: Some(output_id.clone()),
                raw_output,
                model_visible_output: readable_envelope(result.first_window, output_id)?,
            })
        }
        Err(error) => Ok(ManagedToolOutput {
            output_id: None,
            raw_output,
            model_visible_output: degraded_envelope(error)?,
        }),
    }
}

pub(crate) fn readable_envelope(
    window: ToolOutputStoreWindow,
    output_id: ToolOutputId,
) -> Result<ToolOutputEnvelope> {
    match ToolOutputEnvelope::readable_window(
        window.tool_name,
        output_id,
        window.offset,
        window.content,
        window.returned_bytes,
        window.total_bytes,
        window.next_offset,
    ) {
        Ok(envelope) => Ok(envelope),
        Err(source) => Err(Error::OutputEnvelope { source }),
    }
}

pub(crate) fn envelope_json(envelope: &ToolOutputEnvelope, tool_name: &str) -> Result<String> {
    match serde_json::to_string(envelope) {
        Ok(content) => Ok(content),
        Err(source) => Err(Error::EnvelopeSerialization {
            tool_name: tool_name.to_owned(),
            source,
        }),
    }
}

pub(crate) fn envelope_value(envelope: &ToolOutputEnvelope, tool_name: &str) -> Result<Value> {
    match serde_json::to_value(envelope) {
        Ok(content) => Ok(content),
        Err(source) => Err(Error::EnvelopeSerialization {
            tool_name: tool_name.to_owned(),
            source,
        }),
    }
}

fn degraded_envelope(error: ToolOutputStoreError) -> Result<ToolOutputEnvelope> {
    let (reason, window) = match error {
        ToolOutputStoreError::PerOutputOverflow { window, .. } => {
            (ToolOutputRemainderUnavailableReason::OutputTooLarge, window)
        }
        ToolOutputStoreError::AggregateExhausted { window, .. } => (
            ToolOutputRemainderUnavailableReason::BudgetExhausted,
            window,
        ),
        ToolOutputStoreError::WriteFailure { window, .. } => (
            ToolOutputRemainderUnavailableReason::StoreUnavailable,
            window,
        ),
        source => return Err(Error::UnexpectedOutputStoreWrite { source }),
    };
    tracing::warn!(
        tool_name = %window.tool_name,
        total_bytes = window.total_bytes,
        reason = ?reason,
        "tool output remainder is unavailable"
    );
    Ok(ToolOutputEnvelope::degraded_window(
        window.tool_name,
        window.content,
        window.returned_bytes,
        window.total_bytes,
        reason,
    ))
}
