//! Synchronous output preparation and recoverable asynchronous storage.

use ai_interface::{
    SafeErrorSource, ToolOutputEnvelope, ToolOutputId, ToolOutputRemainderUnavailableReason,
};
use serde_json::Value;

use crate::output_store::prefix_window;
use crate::{
    DynToolOutputStore, Error, Result, ToolOutputPolicy, ToolOutputStoreError,
    ToolOutputStoreWindow, ToolOutputWriteRequest,
};

pub(crate) const TOOL_OUTPUT_READ: &str = "tool_output_read";

pub(crate) struct ManagedToolOutput {
    pub(crate) output_id: Option<ToolOutputId>,
    pub(crate) raw_output: Value,
    pub(crate) model_visible_output: ToolOutputEnvelope,
}

/// Keeps a bounded fallback available before a store write can suspend.
pub(crate) struct PreparedToolOutput {
    raw_output: Value,
    serialized: String,
    tool_name: String,
    policy: ToolOutputPolicy,
    pub(crate) fallback: ToolOutputEnvelope,
}

impl PreparedToolOutput {
    pub(crate) async fn finish(self, store: &DynToolOutputStore) -> Result<ManagedToolOutput> {
        if self.serialized.len() <= self.policy.inline_limit_bytes() {
            return Ok(ManagedToolOutput {
                output_id: None,
                raw_output: self.raw_output,
                model_visible_output: self.fallback,
            });
        }
        let total_bytes = self.serialized.len();
        let write = store
            .write(ToolOutputWriteRequest {
                tool_name: self.tool_name.clone(),
                content: self.serialized,
                policy: self.policy,
                first_window_length: self.policy.inline_limit_bytes(),
            })
            .await;
        let (output_id, model_visible_output) = match write {
            Ok(result) => match readable_envelope(result.first_window, result.output_id.clone()) {
                Ok(envelope) => (Some(result.output_id), envelope),
                Err(_) => {
                    warn_degradation(
                        &self.tool_name,
                        total_bytes,
                        &ToolOutputRemainderUnavailableReason::StoreUnavailable,
                    );
                    (None, self.fallback)
                }
            },
            Err(error) => {
                let reason = match error {
                    ToolOutputStoreError::PerOutputOverflow { .. } => {
                        ToolOutputRemainderUnavailableReason::OutputTooLarge
                    }
                    ToolOutputStoreError::AggregateExhausted { .. } => {
                        ToolOutputRemainderUnavailableReason::BudgetExhausted
                    }
                    ToolOutputStoreError::WriteFailure { .. } => {
                        ToolOutputRemainderUnavailableReason::StoreUnavailable
                    }
                    source => {
                        return Err(Error::UnexpectedOutputStoreWrite {
                            source: SafeErrorSource::redacted(source),
                        });
                    }
                };
                warn_degradation(&self.tool_name, total_bytes, &reason);
                (None, replace_remainder_reason(self.fallback, reason))
            }
        };
        Ok(ManagedToolOutput {
            output_id,
            raw_output: self.raw_output,
            model_visible_output,
        })
    }
}

pub(crate) fn prepare_successful_tool_output(
    policy: ToolOutputPolicy,
    tool_name: &str,
    raw_output: Value,
) -> Result<PreparedToolOutput> {
    let serialized = match serde_json::to_string(&raw_output) {
        Ok(serialized) => serialized,
        Err(source) => {
            return Err(Error::OutputSerialization {
                tool_name: tool_name.to_owned(),
                source: SafeErrorSource::redacted(source),
            });
        }
    };
    let total_bytes = serialized.len();
    let fallback = if total_bytes <= policy.inline_limit_bytes() {
        ToolOutputEnvelope::inline(tool_name.to_owned(), raw_output.clone(), total_bytes)
    } else {
        let window = prefix_window(tool_name, &serialized, policy.inline_limit_bytes());
        let reason = if total_bytes > policy.per_output_limit_bytes() {
            ToolOutputRemainderUnavailableReason::OutputTooLarge
        } else {
            ToolOutputRemainderUnavailableReason::StoreUnavailable
        };
        ToolOutputEnvelope::degraded_window(
            window.tool_name,
            window.content,
            window.returned_bytes,
            window.total_bytes,
            reason,
        )
    };
    Ok(PreparedToolOutput {
        raw_output,
        serialized,
        tool_name: tool_name.to_owned(),
        policy,
        fallback,
    })
}

fn replace_remainder_reason(
    envelope: ToolOutputEnvelope,
    reason: ToolOutputRemainderUnavailableReason,
) -> ToolOutputEnvelope {
    match envelope {
        ToolOutputEnvelope::Window(window) => ToolOutputEnvelope::degraded_window(
            window.tool_name(),
            window.content(),
            window.returned_bytes(),
            window.total_bytes(),
            reason,
        ),
        inline => inline,
    }
}

fn warn_degradation(
    tool_name: &str,
    total_bytes: usize,
    reason: &ToolOutputRemainderUnavailableReason,
) {
    tracing::warn!(tool_name, total_bytes, reason = ?reason, "tool output remainder is unavailable");
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
        Err(source) => Err(Error::OutputEnvelope {
            source: SafeErrorSource::redacted(source),
        }),
    }
}

pub(crate) fn envelope_json(envelope: &ToolOutputEnvelope, tool_name: &str) -> Result<String> {
    match serde_json::to_string(envelope) {
        Ok(content) => Ok(content),
        Err(source) => Err(Error::EnvelopeSerialization {
            tool_name: tool_name.to_owned(),
            source: SafeErrorSource::redacted(source),
        }),
    }
}

pub(crate) fn envelope_value(envelope: &ToolOutputEnvelope, tool_name: &str) -> Result<Value> {
    match serde_json::to_value(envelope) {
        Ok(content) => Ok(content),
        Err(source) => Err(Error::EnvelopeSerialization {
            tool_name: tool_name.to_owned(),
            source: SafeErrorSource::redacted(source),
        }),
    }
}
