//! Event reduction and terminal response construction.

use crate::{
    FinishReason, ModelAttempt, ModelError, ModelEvent, ModelResponse, ModelResult, ModelUsage,
};

use super::{
    order_error::ModelEventOrderError,
    part::{DeltaKind, Part},
};

#[derive(Debug, Default)]
/// Incremental fold shared by buffered callers and stream consumers.
pub struct ModelResponseFold {
    parts: Vec<Part>,
    usage: Option<ModelUsage>,
    response: Option<ModelResponse>,
    attempts: Vec<ModelAttempt>,
}

impl ModelResponseFold {
    /// Creates an empty response fold.
    pub fn new() -> Self {
        Self::default()
    }

    /// Applies one event and rejects an event-order violation as an internal error.
    pub fn push(&mut self, event: ModelEvent) -> ModelResult<()> {
        if self.response.is_some() {
            return Err(ModelError::internal(ModelEventOrderError::AfterFinish));
        }
        if self.usage.is_some() && !matches!(&event, ModelEvent::Finish { .. }) {
            return Err(ModelError::internal(ModelEventOrderError::AfterUsage));
        }
        match event {
            ModelEvent::TextDelta { part_index, delta } => {
                self.push_delta(part_index, &delta, DeltaKind::Text)?;
            }
            ModelEvent::ReasoningDelta { part_index, delta } => {
                self.push_delta(part_index, &delta, DeltaKind::Reasoning)?;
            }
            ModelEvent::ToolCallStart {
                part_index,
                id,
                name,
                operation_id,
            } => {
                self.check_new_position(part_index)?;
                self.parts.push(Part::ToolCall {
                    id,
                    name,
                    operation_id,
                    arguments: String::new(),
                    ended: false,
                });
            }
            ModelEvent::ToolCallArgumentsDelta { part_index, delta } => {
                if delta.is_empty() {
                    return Err(ModelError::internal(ModelEventOrderError::EmptyDelta));
                }
                self.existing_part(part_index)?
                    .push_arguments(part_index, &delta)?;
            }
            ModelEvent::ToolCallEnd { part_index } => {
                self.existing_part(part_index)?.end_tool_call(part_index)?;
            }
            ModelEvent::Usage { usage } => {
                if self.parts.iter().any(Part::is_open_tool_call) {
                    return Err(ModelError::internal(ModelEventOrderError::OpenToolCall));
                }
                self.usage = Some(usage);
            }
            ModelEvent::Finish {
                finish_reason,
                provider,
                model_id,
                catalog_model_id,
                thinking_level,
                structured_output,
                provider_context,
            } => {
                let usage = self
                    .usage
                    .take()
                    .ok_or_else(|| ModelError::internal(ModelEventOrderError::MissingUsage))?;
                let mut assistant_message = String::new();
                let mut tool_calls = Vec::new();
                for part in std::mem::take(&mut self.parts) {
                    match part {
                        Part::Text(text) => assistant_message.push_str(&text),
                        part if finish_reason == FinishReason::ToolCalls => {
                            if let Some(call) = part.into_tool_call()? {
                                tool_calls.push(call);
                            }
                        }
                        _ => {}
                    }
                }
                if finish_reason == FinishReason::ToolCalls && tool_calls.is_empty() {
                    return Err(ModelError::internal(ModelEventOrderError::MissingToolCalls));
                }
                self.response = Some(ModelResponse {
                    provider,
                    model_id,
                    catalog_model_id,
                    thinking_level,
                    assistant_message,
                    tool_calls,
                    finish_reason,
                    structured_output,
                    provider_context,
                    usage,
                    attempts: std::mem::take(&mut self.attempts),
                });
            }
            ModelEvent::Fallback { error } => {
                self.parts.clear();
                self.attempts.push(ModelAttempt::from(&error));
            }
        }
        Ok(())
    }

    /// Returns the response after usage, finish, and stream exhaustion.
    pub fn finish(self) -> ModelResult<ModelResponse> {
        self.response
            .ok_or_else(|| ModelError::internal(ModelEventOrderError::MissingFinish))
    }

    fn check_new_position(&self, part_index: usize) -> ModelResult<()> {
        if part_index != self.parts.len() {
            return Err(ModelError::internal(ModelEventOrderError::PartPosition {
                expected: self.parts.len(),
                actual: part_index,
            }));
        }
        Ok(())
    }

    fn existing_part(&mut self, part_index: usize) -> ModelResult<&mut Part> {
        let expected = self.parts.len();
        self.parts.get_mut(part_index).ok_or_else(|| {
            ModelError::internal(ModelEventOrderError::PartPosition {
                expected,
                actual: part_index,
            })
        })
    }

    fn push_delta(&mut self, part_index: usize, delta: &str, kind: DeltaKind) -> ModelResult<()> {
        if delta.is_empty() {
            return Err(ModelError::internal(ModelEventOrderError::EmptyDelta));
        }
        if part_index == self.parts.len() {
            self.parts.push(Part::new_delta(kind));
        }
        self.existing_part(part_index)?
            .push_delta(part_index, delta, kind)
    }
}
