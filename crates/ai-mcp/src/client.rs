//! Public MCP client boundary and retained runtime state.

use std::sync::{
    Arc,
    atomic::{AtomicBool, AtomicU64},
};

use async_trait::async_trait;
use json_http::DynJsonHttpAuth;
use serde_json::Value;
use tokio::sync::Mutex;

use crate::{
    DynMcpHttpTransport, McpServerConfig, McpServerHandshake, McpToolCallOutcome,
    McpToolDescriptor, Result,
};

pub(crate) const LATEST_PROTOCOL_VERSION: &str = "2025-06-18";
pub(crate) const COMPATIBLE_PROTOCOL_VERSION: &str = "2025-03-26";

/// Shared dynamic MCP protocol client.
pub type DynMcpClient = Arc<dyn McpClient>;

#[cfg_attr(
    any(test, doctest, feature = "test-support"),
    unimock::unimock(api = McpClientMock)
)]
#[async_trait]
/// Protocol-focused MCP client boundary.
pub trait McpClient: Send + Sync {
    /// Idempotently initializes the server connection.
    async fn ensure_initialized(&self) -> Result<McpServerHandshake>;

    /// Returns the complete paginated server tool list.
    async fn list_tools(&self) -> Result<Vec<McpToolDescriptor>>;

    /// Calls a server tool by its original unprefixed name.
    async fn call_tool(&self, name: &str, arguments: Value) -> Result<McpToolCallOutcome>;

    /// Reports whether a tool-list invalidation was observed.
    fn tools_list_changed(&self) -> bool;

    /// Terminates the active server session when one exists.
    async fn close(&self) -> Result<()>;
}

#[derive(Clone, Default)]
pub(crate) struct ClientState {
    pub(crate) handshake: Option<McpServerHandshake>,
    pub(crate) session_id: Option<String>,
}

#[derive(Clone)]
pub(crate) struct RequestContext {
    pub(crate) session_id: Option<String>,
    pub(crate) protocol_version: Option<String>,
}

/// Concurrent streamable HTTP implementation of [`McpClient`].
pub struct StreamableHttpMcpClient {
    pub(crate) transport: DynMcpHttpTransport,
    pub(crate) auth: DynJsonHttpAuth,
    pub(crate) config: McpServerConfig,
    pub(crate) state: Mutex<ClientState>,
    pub(crate) initialization_lock: Mutex<()>,
    pub(crate) next_request_id: AtomicU64,
    pub(crate) tools_stale: AtomicBool,
}

impl StreamableHttpMcpClient {
    /// Builds a client after validating its pure connection configuration.
    pub fn new(
        transport: DynMcpHttpTransport,
        auth: DynJsonHttpAuth,
        config: McpServerConfig,
    ) -> Result<Self> {
        config.validate()?;
        Ok(Self {
            transport,
            auth,
            config,
            state: Mutex::new(ClientState::default()),
            initialization_lock: Mutex::new(()),
            next_request_id: AtomicU64::new(1),
            tools_stale: AtomicBool::new(false),
        })
    }
}
