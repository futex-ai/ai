//! Legacy assistant-function and paired-result replay.

use ai_interface::{ConversationMessage, ConversationPart, ConversationRole};

#[derive(Clone, Debug)]
pub(super) struct LegacyFunctionCall {
    pub(super) tool_call_id: String,
    pub(super) name: String,
    pub(super) arguments: String,
}

pub(super) fn legacy_tool_name<'a>(
    message: &ConversationMessage,
    legacy_calls: &'a [LegacyFunctionCall],
) -> Option<&'a str> {
    if message.role != ConversationRole::Tool {
        return None;
    }
    let ConversationPart::ToolResult { call_id, .. } = message.parts.first()? else {
        return None;
    };
    legacy_calls
        .iter()
        .rev()
        .find(|call| call.tool_call_id == *call_id)
        .map(|call| call.name.as_str())
}
