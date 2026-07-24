//! Failure helpers for turn execution.

use ai_interface::{
    ConversationMessage, ModelError, ModelResponse, ToolCall, ToolCallLogResult, ToolError,
};
use serde_json::json;

use crate::{Error, ToolOutputStoreError};

const INVALID_OUTPUT_READ_ARGUMENTS: &str = "Invalid tool output read arguments. Provide an output id and optional non-negative byte offset and positive byte length.";
const UNAVAILABLE_OUTPUT: &str = "The output is no longer available; the original tool call itself succeeded. Re-run the original tool only if it is read-only or otherwise safe to repeat, and confirm with the user before repeating a side-effecting call.";
const UNREADABLE_OUTPUT: &str = "The tool output could not be read. Try again later.";

pub(super) fn tool_error_log_result(error: &ToolError) -> ToolCallLogResult {
    ToolCallLogResult::Error {
        message: error.to_string(),
        debug: format!("{error:?}"),
    }
}

pub(super) fn provider_error(response: &ModelResponse, message: impl Into<String>) -> Error {
    Error::Model(ModelError::provider(
        response.provider.clone(),
        response.model_id.clone(),
        message,
    ))
}

pub(super) fn tool_error_message(call: &ToolCall, error: &ToolError) -> ConversationMessage {
    tool_error_message_with_text(call, error.to_string())
}

pub(super) fn tool_output_read_error_message(
    call: &ToolCall,
    error: &ToolError,
) -> ConversationMessage {
    let message = match error {
        ToolError::InvalidArguments { .. } => INVALID_OUTPUT_READ_ARGUMENTS.to_owned(),
        ToolError::Execution { source, .. } => {
            match source.downcast_ref::<ToolOutputStoreError>() {
                Some(store_error) => output_store_read_error_message(store_error),
                None => unreadable_output_message(),
            }
        }
        ToolError::UnknownTool { .. } => unreadable_output_message(),
    };
    tool_error_message_with_text(call, message)
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
        ToolOutputStoreError::PerOutputOverflow { .. }
        | ToolOutputStoreError::AggregateExhausted { .. }
        | ToolOutputStoreError::WriteFailure { .. } => unreadable_output_message(),
    }
}

fn unreadable_output_message() -> String {
    UNREADABLE_OUTPUT.to_owned()
}

fn tool_error_message_with_text(call: &ToolCall, message: String) -> ConversationMessage {
    ConversationMessage::tool(
        json!({
            "ok": false,
            "error": {
                "message": message,
            }
        })
        .to_string(),
        call.name.clone(),
        call.id.clone(),
    )
}
