//! Typed causes for violations of the model event contract.

use thiserror::Error;

#[derive(Debug, Error)]
pub(super) enum ModelEventOrderError {
    #[error("[ai_interface/model_stream] event after finish")]
    AfterFinish,
    #[error("[ai_interface/model_stream] usage must be followed by finish")]
    AfterUsage,
    #[error("[ai_interface/model_stream] new part {actual} must use position {expected}")]
    PartPosition { expected: usize, actual: usize },
    #[error("[ai_interface/model_stream] part event changed the kind of part {part_index}")]
    PartKind { part_index: usize },
    #[error("[ai_interface/model_stream] empty content delta")]
    EmptyDelta,
    #[error("[ai_interface/model_stream] tool-call event has no active call at part {part_index}")]
    InactiveToolCall { part_index: usize },
    #[error("[ai_interface/model_stream] usage arrived before every tool call ended")]
    OpenToolCall,
    #[error("[ai_interface/model_stream] finish arrived before usage")]
    MissingUsage,
    #[error("[ai_interface/model_stream] stream ended before finish")]
    MissingFinish,
    #[error("[ai_interface/model_stream] tool-calls finish has no completed tool call")]
    MissingToolCalls,
}
