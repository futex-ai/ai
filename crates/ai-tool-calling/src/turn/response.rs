//! Response finish validation and terminal outcome construction.

use ai_interface::{FinishReason, ModelResponse};

use crate::{Result, RunOutcome};

use super::failures::invalid_output_error;

pub(super) fn validate_response(response: &ModelResponse) -> Result<()> {
    if response.finish_reason != FinishReason::ToolCalls && !response.tool_calls().is_empty() {
        return Err(invalid_output_error(
            response,
            "model returned tool calls without a tool-call finish reason",
        ));
    }
    match &response.finish_reason {
        FinishReason::Stop | FinishReason::Truncated => Ok(()),
        FinishReason::Filtered => Err(invalid_output_error(
            response,
            "model response was filtered by the provider",
        )),
        FinishReason::Other(raw) => Err(invalid_output_error(response, raw.clone())),
        FinishReason::ToolCalls if response.tool_calls().is_empty() => Err(invalid_output_error(
            response,
            "model reported tool calls without any tool call payloads",
        )),
        FinishReason::ToolCalls => Ok(()),
    }
}

pub(super) fn response_outcome(
    response: &ModelResponse,
    assistant_message: &str,
    steps_taken: usize,
) -> Option<RunOutcome> {
    match response.finish_reason {
        FinishReason::Stop => Some(RunOutcome::Completed {
            assistant_message: assistant_message.to_owned(),
            steps_taken,
            structured_output: response.structured_output.clone(),
        }),
        FinishReason::Truncated => Some(RunOutcome::Truncated {
            assistant_message: assistant_message.to_owned(),
            steps_taken,
        }),
        _ => None,
    }
}
