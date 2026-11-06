//! Helpers that inspect the public agent event stream.

use ai_interface::{ConversationMessage, ToolError};

use crate::{
    AgentEvent, AgentRequest, DynAgentEventStream, Result, RunOutcome, ToolExecutionRecord,
};

/// Builds the common system prompt and optional model-call budget.
pub(super) fn request(max_steps: Option<usize>) -> AgentRequest {
    AgentRequest {
        system_prompt: "system prompt".to_owned(),
        max_steps,
        ..Default::default()
    }
}

/// Collects unchanged public events until the stream ends or fails.
pub(super) async fn collect_events(mut stream: DynAgentEventStream<'_>) -> Result<Vec<AgentEvent>> {
    let mut events = Vec::new();
    while let Some(event) = stream.next().await? {
        events.push(event);
    }
    Ok(events)
}

/// Returns the final outcome and rejects streams without a finished event.
pub(super) fn run_outcome(events: &[AgentEvent]) -> RunOutcome {
    let Some(AgentEvent::Finished(outcome)) = events.last() else {
        panic!("stream should finish with an outcome");
    };
    outcome.clone()
}

/// Borrows the successful execution records emitted by the runtime.
pub(super) fn executions(events: &[AgentEvent]) -> Vec<&ToolExecutionRecord> {
    events
        .iter()
        .filter_map(|event| match event {
            AgentEvent::ToolFinished {
                result: Ok(record), ..
            } => Some(record),
            _ => None,
        })
        .collect()
}

/// Borrows recoverable tool errors from tool-finished events.
pub(super) fn tool_errors(events: &[AgentEvent]) -> Vec<&ToolError> {
    events
        .iter()
        .filter_map(|event| match event {
            AgentEvent::ToolFinished {
                result: Err(error), ..
            } => Some(error),
            _ => None,
        })
        .collect()
}

/// Borrows each completed step's messages in their conversation order.
pub(super) fn steps(events: &[AgentEvent]) -> Vec<&[ConversationMessage]> {
    events
        .iter()
        .filter_map(|event| match event {
            AgentEvent::StepFinished { messages } => Some(messages.as_slice()),
            _ => None,
        })
        .collect()
}
