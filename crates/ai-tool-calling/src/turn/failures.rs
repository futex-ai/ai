//! Failure helpers for turn execution.

use ai_interface::{
    ConversationMessage, ModelError, ModelResponse, ToolCall, ToolCallLogResult, ToolError,
};
use serde_json::json;

use crate::Error;

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
    ConversationMessage::tool(
        json!({
            "ok": false,
            "error": {
                "message": error.to_string(),
            }
        })
        .to_string(),
        call.name.clone(),
        call.id.clone(),
    )
}
