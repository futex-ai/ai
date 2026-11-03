//! Safe observer callbacks whose failures never interrupt a turn.

use ai_interface::{
    DynLogger, LoggerResult, ModelCallLogEntry, ModelCallLogResult, ModelRequest, ModelResponse,
    SafeErrorSource, ToolActivityLogEntry, ToolActivityPhase, ToolCall, ToolCallLogEntry,
    TurnOutcome, TurnOutcomeLogEntry,
};

use crate::intrinsic::is_intrinsic_tool;
use crate::runtime::RuntimeConfig;
use crate::{Error, RunOutcome};

pub(super) fn log_model_success(
    logger: &DynLogger,
    request: &ModelRequest,
    response: &ModelResponse,
    latency_ms: u128,
) {
    warn_failure(logger.log_model_call(&ModelCallLogEntry {
        request: request.log_copy(),
        result: ModelCallLogResult::Success {
            response: Box::new(response.log_copy()),
        },
        latency_ms,
    }));
}

pub(super) fn log_model_error(
    logger: &DynLogger,
    request: &ModelRequest,
    error: &Error,
    latency_ms: u128,
) {
    warn_failure(logger.log_model_call(&ModelCallLogEntry {
        request: request.log_copy(),
        result: ModelCallLogResult::Error {
            message: error.to_string(),
            debug: format!("{error:?}"),
        },
        latency_ms,
    }));
}

pub(super) fn log_tool_call(logger: &DynLogger, entry: &ToolCallLogEntry) {
    warn_failure(logger.log_tool_call(entry));
}

pub(super) fn log_tool_activity(
    logger: &DynLogger,
    config: &RuntimeConfig,
    call: &ToolCall,
    phase: ToolActivityPhase,
) {
    let activity_verb = config
        .tools
        .activity_verbs_by_name
        .get(&call.name)
        .cloned()
        .or_else(|| is_intrinsic_tool(&call.name).then(|| "Reading".to_owned()));
    warn_failure(logger.log_tool_activity(&ToolActivityLogEntry {
        tool_name: call.name.clone(),
        activity_verb,
        phase,
    }));
}

pub(super) fn log_turn_outcome(logger: &DynLogger, outcome: &RunOutcome, max_steps: Option<usize>) {
    let (assistant_message, steps_taken, outcome) = match outcome {
        RunOutcome::Completed {
            assistant_message,
            steps_taken,
            ..
        } => (assistant_message, steps_taken, TurnOutcome::Completed),
        RunOutcome::Truncated {
            assistant_message,
            steps_taken,
        } => (assistant_message, steps_taken, TurnOutcome::Truncated),
        RunOutcome::Capped {
            assistant_message,
            steps_taken,
            ..
        } => (assistant_message, steps_taken, TurnOutcome::Capped),
    };
    warn_failure(logger.log_turn_outcome(&TurnOutcomeLogEntry {
        assistant_message: assistant_message.clone(),
        steps_taken: *steps_taken,
        outcome,
        max_steps,
    }));
}

fn warn_failure(result: LoggerResult<()>) {
    if let Err(source) = result {
        let source = SafeErrorSource::redacted(source);
        tracing::warn!(error = %source, "agent observer callback failed");
    }
}
