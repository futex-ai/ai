//! Observer callback helpers for tool activity.

use ai_interface::{
    DynLogger, ToolActivityLogEntry, ToolActivityPhase, ToolCall, TurnOutcome, TurnOutcomeLogEntry,
};

use crate::runtime::RuntimeConfig;
use crate::{Result, RunOutcome};

pub(super) fn log_tool_activity(
    logger: &DynLogger,
    config: &RuntimeConfig,
    call: &ToolCall,
    phase: ToolActivityPhase,
) -> Result<()> {
    let activity_verb = config.tools.activity_verbs_by_name.get(&call.name).cloned();
    logger.log_tool_activity(&ToolActivityLogEntry {
        tool_name: call.name.clone(),
        activity_verb,
        phase,
    })?;
    Ok(())
}

pub(super) fn log_turn_outcome(
    logger: &DynLogger,
    outcome: &RunOutcome,
    max_steps: Option<usize>,
) -> Result<()> {
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
    logger.log_turn_outcome(&TurnOutcomeLogEntry {
        assistant_message: assistant_message.clone(),
        steps_taken: *steps_taken,
        outcome,
        max_steps,
    })?;
    Ok(())
}
