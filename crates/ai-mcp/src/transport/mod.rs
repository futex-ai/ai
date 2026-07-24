//! MCP streamable HTTP transport boundaries.

use std::{collections::BTreeMap, sync::Arc, time::Duration};

use async_trait::async_trait;
use serde_json::Value;

use crate::Result;

mod reqwest;
pub(crate) mod sse;

pub use reqwest::ReqwestMcpHttpTransport;

/// Shared dynamic MCP HTTP transport.
pub type DynMcpHttpTransport = Arc<dyn McpHttpTransport>;

/// Single-owner decoded event stream.
pub type DynMcpEventStream = Box<dyn McpEventStream>;

#[cfg_attr(
    any(test, doctest, feature = "test-support"),
    unimock::unimock(api = McpHttpTransportMock)
)]
#[async_trait]
/// Low-level HTTP boundary for the streamable MCP transport.
pub trait McpHttpTransport: Send + Sync {
    /// Sends one JSON-RPC message as an HTTP POST.
    async fn post(
        &self,
        url: &str,
        headers: &BTreeMap<String, String>,
        body: &Value,
        max_response_bytes: usize,
        timeout: Duration,
    ) -> Result<McpHttpResponse>;

    /// Sends one session-termination HTTP DELETE.
    async fn delete(
        &self,
        url: &str,
        headers: &BTreeMap<String, String>,
        max_response_bytes: usize,
        timeout: Duration,
    ) -> Result<McpHttpResponse>;
}

#[cfg_attr(
    any(test, doctest, feature = "test-support"),
    unimock::unimock(api = McpEventStreamMock)
)]
#[async_trait]
/// Pull-based decoded SSE stream scoped to one HTTP response.
pub trait McpEventStream: Send {
    /// Returns the next JSON-RPC message as soon as one SSE event completes.
    async fn next_message(&mut self) -> Result<Option<Value>>;
}

/// HTTP response status, headers, and decoded body.
pub struct McpHttpResponse {
    /// HTTP status code.
    pub status: u16,
    /// Lowercase response-header names mapped to every value in wire order.
    pub headers: BTreeMap<String, Vec<String>>,
    /// Decoded response body.
    pub payload: McpHttpPayload,
}

/// Body representation returned across the MCP transport seam.
pub enum McpHttpPayload {
    /// Empty body, such as an accepted notification.
    None,
    /// One buffered JSON value or textual error body.
    Json(Value),
    /// Live pull-based SSE body.
    EventStream(DynMcpEventStream),
}
