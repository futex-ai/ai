//! Ordered content retained while folding one attempt.

use crate::{ModelError, ModelResult, ToolCall};

use super::order_error::ModelEventOrderError;

#[derive(Debug)]
pub(super) enum Part {
    Text(String),
    Reasoning(String),
    ToolCall {
        id: String,
        name: String,
        operation_id: Option<String>,
        arguments: String,
        ended: bool,
    },
}

#[derive(Clone, Copy)]
pub(super) enum DeltaKind {
    Text,
    Reasoning,
}

impl Part {
    pub(super) fn new_delta(kind: DeltaKind) -> Self {
        match kind {
            DeltaKind::Text => Self::Text(String::new()),
            DeltaKind::Reasoning => Self::Reasoning(String::new()),
        }
    }

    pub(super) fn push_delta(
        &mut self,
        part_index: usize,
        delta: &str,
        kind: DeltaKind,
    ) -> ModelResult<()> {
        match (self, kind) {
            (Self::Text(text), DeltaKind::Text) | (Self::Reasoning(text), DeltaKind::Reasoning) => {
                text.push_str(delta);
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

    pub(super) fn end_tool_call(&mut self, part_index: usize) -> ModelResult<()> {
        match self {
            Self::ToolCall { ended, .. } if !*ended => {
                *ended = true;
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

    pub(super) fn into_tool_call(self) -> ModelResult<Option<ToolCall>> {
        let Self::ToolCall {
            id,
            name,
            operation_id,
            arguments,
            ..
        } = self
        else {
            return Ok(None);
        };
        let input = match serde_json::from_str(&arguments) {
            Ok(input) => input,
            Err(source) => return Err(ModelError::internal(source)),
        };
        Ok(Some(ToolCall {
            id,
            name,
            input,
            operation_id,
        }))
    }
}
