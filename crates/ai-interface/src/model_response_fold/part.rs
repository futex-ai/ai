//! Ordered content retained while folding one attempt.

use crate::{ConversationPart, ModelError, ModelResult, ProviderState};

use super::order_error::ModelEventOrderError;

#[derive(Debug)]
pub(super) enum Part {
    Content(ConversationPart),
    ToolCall {
        id: String,
        name: String,
        operation_id: Option<String>,
        arguments: String,
        ended: bool,
        provider_state: Option<ProviderState>,
    },
}

#[derive(Clone, Copy)]
pub(super) enum DeltaKind {
    Text,
    Reasoning,
}

impl Part {
    pub(super) fn new_delta(kind: DeltaKind) -> Self {
        Self::Content(match kind {
            DeltaKind::Text => ConversationPart::text(""),
            DeltaKind::Reasoning => ConversationPart::reasoning(""),
        })
    }

    pub(super) fn push_delta(
        &mut self,
        part_index: usize,
        delta: &str,
        kind: DeltaKind,
        state: Option<ProviderState>,
    ) -> ModelResult<()> {
        match (self, kind) {
            (
                Self::Content(ConversationPart::Text {
                    text,
                    provider_state,
                }),
                DeltaKind::Text,
            )
            | (
                Self::Content(ConversationPart::Reasoning {
                    text,
                    provider_state,
                }),
                DeltaKind::Reasoning,
            ) => {
                text.push_str(delta);
                if state.is_some() {
                    *provider_state = state;
                }
                Ok(())
            }
            _ => Err(ModelError::internal(ModelEventOrderError::PartKind {
                part_index,
            })),
        }
    }

    pub(super) fn push_arguments(&mut self, part_index: usize, delta: &str) -> ModelResult<()> {
        match self {
            Self::ToolCall {
                arguments,
                ended: false,
                ..
            } => {
                arguments.push_str(delta);
                Ok(())
            }
            _ => Err(ModelError::internal(
                ModelEventOrderError::InactiveToolCall { part_index },
            )),
        }
    }

    pub(super) fn end_tool_call(
        &mut self,
        part_index: usize,
        state: Option<ProviderState>,
    ) -> ModelResult<()> {
        match self {
            Self::ToolCall {
                ended,
                provider_state,
                ..
            } if !*ended => {
                *ended = true;
                *provider_state = state;
                Ok(())
            }
            _ => Err(ModelError::internal(
                ModelEventOrderError::InactiveToolCall { part_index },
            )),
        }
    }

    pub(super) fn is_open_tool_call(&self) -> bool {
        matches!(self, Self::ToolCall { ended: false, .. })
    }

    pub(super) fn into_part(self, keep_tool_calls: bool) -> ModelResult<Option<ConversationPart>> {
        let Self::ToolCall {
            id,
            name,
            operation_id,
            arguments,
            provider_state,
            ..
        } = self
        else {
            let Self::Content(part) = self else {
                return Ok(None);
            };
            return Ok(Some(part));
        };
        if !keep_tool_calls {
            return Ok(None);
        }
        let arguments = match serde_json::from_str(&arguments) {
            Ok(arguments) => arguments,
            Err(source) => return Err(ModelError::internal(source)),
        };
        Ok(Some(ConversationPart::ToolCall {
            call_id: id,
            tool_name: name,
            arguments,
            operation_id,
            provider_state,
        }))
    }
}
