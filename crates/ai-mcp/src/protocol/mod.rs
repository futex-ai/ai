//! Typed MCP JSON-RPC and tools protocol data.

mod content;
mod content_serde;
mod initialize;
mod rpc;
mod tools;

pub use content::{McpAnnotations, McpContentBlock, McpResourceContents, McpRole};
pub use initialize::{
    McpServerCapabilities, McpServerHandshake, McpServerInfo, McpToolsCapability,
};
pub use rpc::McpRequestId;
pub use tools::{McpToolCallOutcome, McpToolDescriptor};

pub(crate) use initialize::InitializeResult;
pub(crate) use rpc::{
    JsonRpcMessageKind, classify_message, error_response, notification, request, success_response,
};
pub(crate) use tools::ListToolsResult;
