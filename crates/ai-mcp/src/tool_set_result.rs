//! MCP result precedence and bounded model-visible output.

use ai_interface::{ToolError, ToolResult};
use serde::Serialize;
use serde_json::{Value, json};

use crate::{McpContentBlock, McpToolCallOutcome};

/// Serialized byte length of the empty success truncation envelope.
pub(crate) const MIN_SUCCESS_RESPONSE_BYTES: usize = 31;
/// Serialized byte length of the empty remote-error truncation envelope.
pub(crate) const MIN_RESPONSE_BYTES: usize = 47;

#[derive(Clone, Copy)]
enum TruncationKind {
    Success,
    RemoteError,
}

#[derive(Serialize)]
struct RemoteErrorResult<'a> {
    content: &'a Value,
    is_error: bool,
}

pub(crate) fn map_outcome(
    tool_name: &str,
    outcome: McpToolCallOutcome,
    max_response_bytes: usize,
) -> ToolResult<Value> {
    if outcome.is_error {
        let content = serialize_content(tool_name, &outcome.content)?;
        return bound_remote_error(tool_name, content, max_response_bytes);
    }
    let mapped = if let Some(structured) = outcome.structured_content {
        structured
    } else if let [McpContentBlock::Text { text, .. }] = outcome.content.as_slice() {
        Value::String(text.clone())
    } else {
        serialize_content(tool_name, &outcome.content)?
    };
    bound_result(tool_name, mapped, max_response_bytes)
}

fn serialize_content(tool_name: &str, content: &[McpContentBlock]) -> ToolResult<Value> {
    match serde_json::to_value(content) {
        Ok(value) => Ok(value),
        Err(source) => Err(ToolError::execution(tool_name, source)),
    }
}

fn bound_result(tool_name: &str, value: Value, max_response_bytes: usize) -> ToolResult<Value> {
    let serialized = match serde_json::to_vec(&value) {
        Ok(serialized) => serialized,
        Err(source) => return Err(ToolError::execution(tool_name, source)),
    };
    if serialized.len() <= max_response_bytes {
        return Ok(value);
    }
    let source = match String::from_utf8(serialized) {
        Ok(source) => source,
        Err(error) => return Err(ToolError::execution(tool_name, error)),
    };
    truncation_envelope(
        tool_name,
        &source,
        max_response_bytes,
        TruncationKind::Success,
    )
}

fn bound_remote_error(
    tool_name: &str,
    content: Value,
    max_response_bytes: usize,
) -> ToolResult<Value> {
    let serialized = match serde_json::to_vec(&RemoteErrorResult {
        content: &content,
        is_error: true,
    }) {
        Ok(serialized) => serialized,
        Err(source) => return Err(ToolError::execution(tool_name, source)),
    };
    if serialized.len() <= max_response_bytes {
        return Ok(json!({"is_error": true, "content": content}));
    }
    let source = match serde_json::to_string(&content) {
        Ok(source) => source,
        Err(error) => return Err(ToolError::execution(tool_name, error)),
    };
    truncation_envelope(
        tool_name,
        &source,
        max_response_bytes,
        TruncationKind::RemoteError,
    )
}

fn truncation_envelope(
    tool_name: &str,
    source: &str,
    max_response_bytes: usize,
    kind: TruncationKind,
) -> ToolResult<Value> {
    let baseline = match kind {
        TruncationKind::Success => MIN_SUCCESS_RESPONSE_BYTES,
        TruncationKind::RemoteError => MIN_RESPONSE_BYTES,
    };
    let available = max_response_bytes.saturating_sub(baseline);
    let mut prefix = String::new();
    let mut escaped_bytes = 0_usize;
    for character in source.chars() {
        let encoded = match serde_json::to_string(&character.to_string()) {
            Ok(encoded) => encoded,
            Err(error) => return Err(ToolError::execution(tool_name, error)),
        };
        let encoded_bytes = encoded.len().saturating_sub(2);
        if escaped_bytes.saturating_add(encoded_bytes) > available {
            break;
        }
        prefix.push(character);
        escaped_bytes += encoded_bytes;
    }
    match kind {
        TruncationKind::Success => Ok(json!({"truncated": true, "content": prefix})),
        TruncationKind::RemoteError => {
            Ok(json!({"is_error": true, "truncated": true, "content": prefix}))
        }
    }
}

#[cfg(test)]
fn empty_truncation_envelope() -> Value {
    json!({"truncated": true, "content": ""})
}

#[cfg(test)]
fn empty_error_truncation_envelope() -> Value {
    json!({"is_error": true, "truncated": true, "content": ""})
}

#[cfg(test)]
#[path = "_tests_/tool_set_result_tests.rs"]
mod tool_set_result_tests;
