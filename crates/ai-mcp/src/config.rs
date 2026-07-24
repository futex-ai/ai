//! Pure configuration for one MCP server connection.

use std::time::Duration;

use crate::{Error, Result};

const DEFAULT_REQUEST_TIMEOUT: Duration = Duration::from_secs(30);
const DEFAULT_TOOL_CALL_TIMEOUT: Duration = Duration::from_secs(120);
const DEFAULT_MAX_RESPONSE_BYTES: usize = 1024 * 1024;

#[derive(Clone, Debug, Eq, PartialEq)]
/// Connection settings for one streamable HTTP MCP server.
pub struct McpServerConfig {
    /// Stable key used when namespacing tools from this server.
    pub server_key: String,
    /// Single streamable HTTP MCP endpoint.
    pub url: String,
    /// Timeout for initialization and tool discovery requests.
    pub request_timeout: Duration,
    /// Timeout for tool calls.
    pub tool_call_timeout: Duration,
    /// Maximum bytes accepted from one HTTP response.
    pub max_response_bytes: usize,
    /// Optional activity label copied onto exposed tool definitions.
    pub activity_verb: Option<String>,
}

impl McpServerConfig {
    /// Builds a configuration with protocol defaults.
    pub fn new(server_key: impl Into<String>, url: impl Into<String>) -> Self {
        Self {
            server_key: server_key.into(),
            url: url.into(),
            request_timeout: DEFAULT_REQUEST_TIMEOUT,
            tool_call_timeout: DEFAULT_TOOL_CALL_TIMEOUT,
            max_response_bytes: DEFAULT_MAX_RESPONSE_BYTES,
            activity_verb: None,
        }
    }

    /// Validates the stable server key and positive limits.
    pub fn validate(&self) -> Result<()> {
        let valid_key = !self.server_key.is_empty()
            && self.server_key.len() <= 32
            && self.server_key.bytes().all(|byte| {
                byte.is_ascii_lowercase() || byte.is_ascii_digit() || b"_-".contains(&byte)
            });
        if !valid_key {
            return Err(Error::InvalidServerKey {
                server_key: self.server_key.clone(),
            });
        }
        if self.max_response_bytes == 0 {
            return Err(Error::InvalidResponseLimit);
        }
        if self.request_timeout.is_zero() || self.tool_call_timeout.is_zero() {
            return Err(Error::InvalidTimeout);
        }
        Ok(())
    }
}
