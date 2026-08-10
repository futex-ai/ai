//! Tool descriptor and call-result wire DTOs.

use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::McpContentBlock;

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
/// One server tool discovered through `tools/list`.
pub struct McpToolDescriptor {
    /// Original server-owned tool name.
    pub name: String,
    /// Optional human-readable title.
    pub title: Option<String>,
    /// Optional model-facing description.
    pub description: Option<String>,
    /// JSON Schema describing accepted arguments.
    pub input_schema: Value,
    /// Optional JSON Schema describing structured output.
    pub output_schema: Option<Value>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
/// Result returned by an MCP `tools/call` request.
pub struct McpToolCallOutcome {
    /// Ordered content blocks returned by the server.
    pub content: Vec<McpContentBlock>,
    /// Optional structured result.
    pub structured_content: Option<Value>,
    /// Whether the remote tool reported an execution error.
    #[serde(default)]
    pub is_error: bool,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ListToolsResult {
    pub(crate) tools: Vec<McpToolDescriptor>,
    pub(crate) next_cursor: Option<String>,
}
