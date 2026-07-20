//! Runtime-owned intrinsic tool definitions.

use ai_interface::ToolDefinition;
use serde_json::json;

use crate::tool_output::TOOL_OUTPUT_READ;

pub(crate) fn tool_output_read_definition() -> ToolDefinition {
    ToolDefinition {
        name: TOOL_OUTPUT_READ.to_owned(),
        description: "Read a byte window from a previously returned large tool output. Read further windows only when the task requires more of that output. Prefer narrowing the original query at its source when a smaller tool result would answer the task. Offsets and lengths are UTF-8 bytes, not tokens.".to_owned(),
        input_schema: json!({
            "type": "object",
            "required": ["output_id"],
            "additionalProperties": false,
            "properties": {
                "output_id": {
                    "type": "string",
                    "description": "Opaque output id from a prior tool_output_window."
                },
                "offset": {
                    "type": "integer",
                    "minimum": 0,
                    "description": "UTF-8 byte offset to read from."
                },
                "length": {
                    "type": "integer",
                    "minimum": 1,
                    "description": "Maximum UTF-8 bytes to return."
                }
            }
        }),
        activity_verb: Some("Reading".to_owned()),
    }
}

pub(crate) fn is_intrinsic_tool(name: &str) -> bool {
    name == TOOL_OUTPUT_READ
}
