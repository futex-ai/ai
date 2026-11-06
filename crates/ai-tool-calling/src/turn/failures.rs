//! Safe model-facing messages for recoverable tool failures.

use ai_interface::{
    ConversationMessage, ModelError, ModelResponse, SafeErrorSource, ToolCall, ToolCallLogResult,
    ToolError,
};
use serde_json::json;

use crate::{Error, ToolOutputStoreError};

const INVALID_OUTPUT_READ_ARGUMENTS: &str = "Invalid tool output read arguments. Provide an output id and optional non-negative byte offset and positive byte length.";
const UNAVAILABLE_OUTPUT: &str = "The output is no longer available; the original tool call itself succeeded. Re-run the original tool only if it is read-only or otherwise safe to repeat, and confirm with the user before repeating a side-effecting call.";
const UNREADABLE_OUTPUT: &str = "The tool output could not be read. Try again later.";

pub(super) fn invalid_output_error(response: &ModelResponse, message: impl Into<String>) -> Error {
    Error::Model(
        ModelError::invalid_output(
            response.provider.clone(),
            response.model_id.clone(),
            message,
        )
        .with_attempts(response.attempts.clone()),
    )
}

pub(super) fn tool_error_log_result(error: &ToolError) -> ToolCallLogResult {
    ToolCallLogResult::Error {
        message: error.to_string(),
        debug: format!("{error:?}"),
    }
}

pub(super) fn tool_error_message(call: &ToolCall, error: &ToolError) -> ConversationMessage {
    let message = if call.name == "tool_output_read" {
        read_error_message(error)
    } else {
        error.to_string()
    };
    ConversationMessage::tool(
        json!({"ok": false, "error": {"message": message}}).to_string(),
        call.name.clone(),
        call.id.clone(),
    )
}

fn read_error_message(error: &ToolError) -> String {
    match error {
        ToolError::InvalidArguments { .. } => INVALID_OUTPUT_READ_ARGUMENTS.to_owned(),
        ToolError::Execution { source, .. } => {
            let cause = match source.downcast_ref::<SafeErrorSource>() {
                Some(safe) => safe.raw_cause(),
                None => source.as_ref(),
            };
            match cause.downcast_ref::<ToolOutputStoreError>() {
                Some(error) => output_store_read_error_message(error),
                None => UNREADABLE_OUTPUT.to_owned(),
            }
        }
        ToolError::UnknownTool { .. } => UNREADABLE_OUTPUT.to_owned(),
    }
}

fn output_store_read_error_message(error: &ToolOutputStoreError) -> String {
    match error {
        ToolOutputStoreError::UnavailableOutput { .. } => UNAVAILABLE_OUTPUT.to_owned(),
        ToolOutputStoreError::InvalidOffset {
            offset,
            total_bytes,
            ..
        } => format!(
            "Invalid offset {offset} for an output of {total_bytes} bytes. Use a UTF-8 byte boundary from 0 through {total_bytes}."
        ),
        ToolOutputStoreError::InvalidLength { length } => {
            format!("Invalid read length {length}; use at least 1 byte.")
        }
        ToolOutputStoreError::NoCompleteCharacterFits {
            offset,
            length,
            minimum_usable_length,
        } => format!(
            "No complete UTF-8 character fits in length {length} at offset {offset}; use at least {minimum_usable_length} bytes."
        ),
        _ => UNREADABLE_OUTPUT.to_owned(),
    }
}

pub(super) fn safe_output_read_source(error: ToolOutputStoreError) -> SafeErrorSource {
    let diagnostic = match &error {
        ToolOutputStoreError::UnavailableOutput { .. } => {
            "[ai_tool_calling/output_store] UnavailableOutput: output is no longer available"
                .to_owned()
        }
        ToolOutputStoreError::InvalidOffset {
            offset,
            total_bytes,
            ..
        } => format!(
            "[ai_tool_calling/output_store] InvalidOffset: offset {offset} for output of {total_bytes} bytes"
        ),
        ToolOutputStoreError::InvalidLength { length } => {
            format!("[ai_tool_calling/output_store] InvalidLength: length {length}; minimum is 1")
        }
        ToolOutputStoreError::NoCompleteCharacterFits {
            offset,
            length,
            minimum_usable_length,
        } => format!(
            "[ai_tool_calling/output_store] NoCompleteCharacterFits: length {length} at offset {offset}; minimum usable length is {minimum_usable_length}"
        ),
        _ => return SafeErrorSource::redacted(error),
    };
    SafeErrorSource::new(error, vec![diagnostic])
}
