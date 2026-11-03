//! Public request and event stream contracts for an agent turn.

use ai_interface::{
    ConversationMessage, ModelCallControls, ModelEvent, ModelResponse, StructuredOutputSchema,
    ToolResult,
};
use async_trait::async_trait;

use crate::{ModelResponseCheckpoint, Result, RunOutcome, ToolExecutionRecord, TurnCheckpoint};

#[derive(Clone, Debug, Default, PartialEq)]
/// Immutable inputs applied to every model step of a turn.
pub struct AgentRequest {
    /// System instructions sent unchanged to each model call.
    pub system_prompt: String,
    /// Generation and execution controls shared by every step.
    pub controls: ModelCallControls,
    /// Maximum model calls, or no cap when absent.
    pub max_steps: Option<usize>,
    /// Required schema for the final answer.
    pub final_answer_schema: Option<StructuredOutputSchema>,
}

#[derive(Debug)]
/// One ordered observation from a running agent turn.
pub enum AgentEvent {
    /// Original model event, including fallback events.
    Model(ModelEvent),
    /// Validated response after the caller's checkpoints.
    Response(ModelResponse),
    /// A tool call starts.
    ToolStarted {
        /// Provider tool-call identifier.
        call_id: String,
        /// Requested model-visible tool name.
        tool_name: String,
    },
    /// A tool call completed with a result or recoverable error.
    ToolFinished {
        /// Provider tool-call identifier.
        call_id: String,
        /// Successful execution record or tool error.
        result: ToolResult<ToolExecutionRecord>,
    },
    /// All model and tool work for a step completed.
    StepFinished {
        /// Messages appended by this step, in conversation order.
        messages: Vec<ConversationMessage>,
    },
    /// Terminal turn outcome. No event follows it.
    Finished(RunOutcome),
}

#[async_trait]
/// Pull-based stream whose lifetime holds the caller's conversation borrow.
pub trait AgentEventStream: Send {
    /// Returns the next event, an error once, or permanent exhaustion.
    async fn next(&mut self) -> Result<Option<AgentEvent>>;
}

/// Owned event stream borrowing the caller's turn state.
pub type DynAgentEventStream<'a> = Box<dyn AgentEventStream + 'a>;

/// Agent boundary for turns over caller-owned conversation history.
pub trait Agent: Send + Sync {
    /// Runs a turn, appending messages to the caller's conversation.
    fn run<'a>(
        &'a self,
        conversation: &'a mut Vec<ConversationMessage>,
        request: AgentRequest,
    ) -> DynAgentEventStream<'a>;

    /// Runs a turn with caller-provided interruption and response hooks.
    fn run_with_checkpoints<'a>(
        &'a self,
        conversation: &'a mut Vec<ConversationMessage>,
        request: AgentRequest,
        checkpoint: &'a mut dyn TurnCheckpoint,
        response_checkpoint: &'a mut dyn ModelResponseCheckpoint,
    ) -> DynAgentEventStream<'a>;
}
