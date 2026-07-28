//! Pure configuration for one MCP server connection.

use std::time::Duration;

use crate::{Error, Result, tool_set_result::MIN_RESPONSE_BYTES};

const DEFAULT_REQUEST_TIMEOUT: Duration = Duration::from_secs(30);
const DEFAULT_TOOL_CALL_TIMEOUT: Duration = Duration::from_secs(120);
const DEFAULT_MAX_RESPONSE_BYTES: usize = 1024 * 1024;
const DEFAULT_MAX_TOOL_PAGES: usize = 100;

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
    /// Maximum bytes accepted from one HTTP response or exposed tool result.
    ///
    /// Must be at least 47 so a truncated remote error retains its marker.
    pub max_response_bytes: usize,
    /// Maximum number of pages accepted from one tool-list operation.
    pub max_tool_pages: usize,
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
            max_tool_pages: DEFAULT_MAX_TOOL_PAGES,
            activity_verb: None,
        }
    }

    /// Validates the stable server key, response and page limits, and timeouts.
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
        if self.max_response_bytes < MIN_RESPONSE_BYTES {
            return Err(Error::InvalidResponseLimit {
                minimum: MIN_RESPONSE_BYTES,
            });
        }
        if self.max_tool_pages == 0 {
            return Err(Error::InvalidToolPageLimit);
        }
        if self.request_timeout.is_zero() || self.tool_call_timeout.is_zero() {
            return Err(Error::InvalidTimeout);
        }
        Ok(())
    }
}
