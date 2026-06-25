//! Public turn outcome and handle contracts.

use ai_interface::ModelResponse;
use async_trait::async_trait;

use crate::Result;

#[derive(Clone, Debug, PartialEq)]
/// One successful turn step outcome.
pub enum StepOutcome {
    /// One model round completed and produced tool calls that were handled successfully.
    Stepped {
        /// Most recent non-empty assistant response seen so far.
        assistant_message: String,
        /// Number of model rounds executed in the turn.
        steps_taken: usize,
        /// Successful tool execution records from this round.
        tool_results: Vec<ToolExecutionRecord>,
    },
    /// The model returned a terminal finish reason and the turn is complete.
    Completed {
        /// Most recent non-empty assistant response seen so far.
        assistant_message: String,
        /// Number of model rounds executed in the turn.
        steps_taken: usize,
    },
    /// The step budget has been exhausted.
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
/// Terminal outcome returned by `run()`.
pub enum RunOutcome {
    /// The model completed with a terminal finish reason.
    Completed {
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
    /// JSON payload returned by the tool implementation.
    pub output: serde_json::Value,
}

#[unimock::unimock(api = TurnCheckpointMock)]
/// Hook checked before model calls, after model responses, and around tool calls.
pub trait TurnCheckpoint: Send {
    /// Fails the current turn when the embedding runtime cannot continue.
    fn check(&mut self) -> Result<()>;
}

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

#[async_trait]
/// Stateful turn handle returned by `send(...)`.
pub trait Turn {
    /// Executes one model round and branches on the normalized finish reason.
    async fn step(&mut self) -> Result<StepOutcome>;

    /// Continues stepping until completion or the configured step cap.
    async fn run(&mut self) -> Result<RunOutcome>;

    /// Returns the number of model rounds already executed by this turn.
    fn steps_taken(&self) -> usize;

    /// Returns successful tool calls observed so far in this turn.
    fn successful_tool_calls(&self) -> &[ToolExecutionRecord];
}
