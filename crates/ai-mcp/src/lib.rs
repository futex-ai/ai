//! Model Context Protocol client and tool adapter for streamable HTTP servers.

#![warn(unreachable_pub)]

mod authorization;
mod client;
mod client_operations;
mod client_request;
mod client_response;
mod config;
mod error;
mod protocol;
mod tool_set;
mod tool_set_naming;
mod tool_set_result;
mod transport;

pub use authorization::{McpAuthorizationChallenge, McpAuthorizationFailure};
#[cfg(any(test, doctest, feature = "test-support"))]
pub use client::McpClientMock;
pub use client::{DynMcpClient, McpClient, StreamableHttpMcpClient};
pub use config::McpServerConfig;
pub use error::{Error, Result};
pub use protocol::{
    McpAnnotations, McpContentBlock, McpRequestId, McpResourceContents, McpRole,
    McpServerCapabilities, McpServerHandshake, McpServerInfo, McpToolCallOutcome,
    McpToolDescriptor, McpToolsCapability,
};
pub use tool_set::McpToolSet;
pub use transport::{
    DynMcpEventStream, DynMcpHttpTransport, McpEventStream, McpHttpPayload, McpHttpResponse,
    McpHttpTransport, ReqwestMcpHttpTransport,
};
#[cfg(any(test, doctest, feature = "test-support"))]
pub use transport::{McpEventStreamMock, McpHttpTransportMock};

#[cfg(test)]
#[path = "_tests_/mod.rs"]
mod tests;
