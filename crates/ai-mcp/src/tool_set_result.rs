//! MCP result precedence and bounded model-visible output.

use ai_interface::{ToolError, ToolResult};
use serde_json::{Value, json};

use crate::{McpContentBlock, McpToolCallOutcome};

pub(crate) fn map_outcome(
    tool_name: &str,
    outcome: McpToolCallOutcome,
    max_response_bytes: usize,
) -> ToolResult<Value> {
    let content = serialize_content(tool_name, &outcome.content)?;
    let mapped = if outcome.is_error {
        json!({"is_error": true, "content": content})
    } else if let Some(structured) = outcome.structured_content {
        structured
    } else if let [McpContentBlock::Text { text, .. }] = outcome.content.as_slice() {
        Value::String(text.clone())
    } else {
        content
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
    truncation_envelope(tool_name, &source, max_response_bytes)
}

fn truncation_envelope(
    tool_name: &str,
    source: &str,
    max_response_bytes: usize,
) -> ToolResult<Value> {
    let empty = json!({"truncated": true, "content": ""});
    let baseline = serialized_len(tool_name, &empty)?;
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
    Ok(json!({"truncated": true, "content": prefix}))
}

fn serialized_len(tool_name: &str, value: &Value) -> ToolResult<usize> {
    match serde_json::to_vec(value) {
        Ok(serialized) => Ok(serialized.len()),
        Err(source) => Err(ToolError::execution(tool_name, source)),
    }
}
