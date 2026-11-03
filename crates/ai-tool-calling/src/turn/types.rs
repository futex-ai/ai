//! Public turn outcome and handle contracts.

use ai_interface::{ModelResponse, ToolOutputEnvelope, ToolOutputId};

use crate::Result;

#[derive(Clone, Debug, PartialEq)]
/// Terminal outcome returned by `run()`.
pub enum RunOutcome {
    /// The model stopped normally.
    Completed {
        /// Parsed final output when a response schema was requested.
        structured_output: Option<serde_json::Value>,
        /// Most recent non-empty assistant response seen so far.
        assistant_message: String,
        /// Number of model rounds executed in the turn.
        steps_taken: usize,
    },
    /// A token or context limit cut off the model response.
    Truncated {
        /// Most recent non-empty assistant response seen so far.
        assistant_message: String,
        /// Number of model rounds executed in the turn.
        steps_taken: usize,
    },
    /// The runtime exhausted the step budget before completion.
    Capped {
        /// Most recent non-empty assistant response seen so far.
        assistant_message: String,
        /// Number of model rounds executed in the turn.
        steps_taken: usize,
        /// Configured maximum step count.
        max_steps: usize,
    },
}

#[derive(Clone, Debug, PartialEq)]
/// Successful tool execution record captured by the runtime.
pub struct ToolExecutionRecord {
    /// Tool call identifier associated with the output.
    pub call_id: String,
    /// Runtime operation id used as the tool idempotency key.
    pub operation_id: String,
    /// Tool name associated with the output.
    pub name: String,
    /// Stored output id when the output can be read through `tool_output_read`.
    pub output_id: Option<ToolOutputId>,
    /// Raw JSON payload returned by the tool implementation.
    pub raw_output: serde_json::Value,
    /// Bounded model-visible envelope appended to conversation and logs.
    pub model_visible_output: ToolOutputEnvelope,
}

#[cfg_attr(
    any(test, doctest, feature = "test-support"),
    unimock::unimock(api = TurnCheckpointMock)
)]
/// Hook checked before model calls, after model responses, and around tool calls.
pub trait TurnCheckpoint: Send {
    /// Fails the current turn when the embedding runtime cannot continue.
    fn check(&mut self) -> Result<()>;
}

#[cfg_attr(
    any(test, doctest, feature = "test-support"),
    unimock::unimock(api = ModelResponseCheckpointMock)
)]
/// Hook called after provider response validation and before tool dispatch.
pub trait ModelResponseCheckpoint: Send {
    /// Persists or observes one validated model response.
    fn checkpoint_response(&mut self, response: &mut ModelResponse) -> Result<()>;
}

/// Checkpoint that never interrupts.
pub struct NoopTurnCheckpoint;

impl TurnCheckpoint for NoopTurnCheckpoint {
    fn check(&mut self) -> Result<()> {
        Ok(())
    }
}

/// Response checkpoint that does no work.
pub struct NoopModelResponseCheckpoint;

impl ModelResponseCheckpoint for NoopModelResponseCheckpoint {
    fn checkpoint_response(&mut self, _response: &mut ModelResponse) -> Result<()> {
        Ok(())
    }
}
