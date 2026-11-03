//! Public deterministic scope and call-id construction.

use ai_interface::ModelRequest;

use super::hasher::StableHasher;

/// Builds a deterministic scope for provider-synthesized tool-call ids.
pub fn synthetic_tool_call_scope(request: &ModelRequest) -> String {
    let mut hasher = StableHasher::new();
    hasher.write_str(&request.system_prompt);
    hasher.write_usize(request.messages.len());
    for message in &request.messages {
        hasher.write_message(message);
    }
    hasher.write_usize(request.tools.len());
    for tool in &request.tools {
        hasher.write_tool(tool);
    }
    hasher.write_optional_schema(request.response_schema.as_ref());
    hasher.finish_hex()
}

/// Builds a deterministic local id for a tool call with no provider id.
pub fn synthetic_tool_call_id(
    prefix: &str,
    request_scope: &str,
    call_index: usize,
    name: &str,
    arguments: &str,
) -> String {
    let mut hasher = StableHasher::new();
    hasher.write_str(prefix);
    hasher.write_str(request_scope);
    hasher.write_usize(call_index);
    hasher.write_str(name);
    hasher.write_str(arguments);
    format!("{prefix}{}", hasher.finish_hex())
}
