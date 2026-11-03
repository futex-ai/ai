//! Copies event values into the shared fold while preserving the original event.

use ai_interface::{ModelError, ModelEvent, SafeErrorSource};
use thiserror::Error;

use crate::{Error, Result};

pub(super) fn copy_for_fold(event: &ModelEvent) -> Result<ModelEvent> {
    Ok(match event {
        ModelEvent::TextDelta {
            part_index,
            delta,
            provider_state,
        } => ModelEvent::TextDelta {
            part_index: *part_index,
            delta: delta.clone(),
            provider_state: provider_state.clone(),
        },
        ModelEvent::ReasoningDelta {
            part_index,
            delta,
            provider_state,
        } => ModelEvent::ReasoningDelta {
            part_index: *part_index,
            delta: delta.clone(),
            provider_state: provider_state.clone(),
        },
        ModelEvent::ToolCallStart {
            part_index,
            id,
            name,
            operation_id,
        } => ModelEvent::ToolCallStart {
            part_index: *part_index,
            id: id.clone(),
            name: name.clone(),
            operation_id: operation_id.clone(),
        },
        ModelEvent::ToolCallArgumentsDelta { part_index, delta } => {
            ModelEvent::ToolCallArgumentsDelta {
                part_index: *part_index,
                delta: delta.clone(),
            }
        }
        ModelEvent::ToolCallEnd {
            part_index,
            provider_state,
        } => ModelEvent::ToolCallEnd {
            part_index: *part_index,
            provider_state: provider_state.clone(),
        },
        ModelEvent::Usage { usage } => ModelEvent::Usage {
            usage: usage.clone(),
        },
        ModelEvent::Finish {
            finish_reason,
            provider,
            model_id,
            catalog_model_id,
            thinking_level,
            structured_output,
        } => ModelEvent::Finish {
            finish_reason: finish_reason.clone(),
            provider: provider.clone(),
            model_id: model_id.clone(),
            catalog_model_id: catalog_model_id.clone(),
            thinking_level: *thinking_level,
            structured_output: structured_output.clone(),
        },
        ModelEvent::Fallback {
            error,
            emitted_events,
        } => ModelEvent::Fallback {
            error: ModelError::new(error.core().clone()).with_attempts(error.attempts().to_vec()),
            emitted_events: *emitted_events,
        },
        _ => {
            return Err(Error::Internal {
                source: SafeErrorSource::redacted(UnknownModelEvent),
            });
        }
    })
}

#[derive(Debug, Error)]
#[error("[ai_tool_calling/turn] unrecognized model event")]
struct UnknownModelEvent;
