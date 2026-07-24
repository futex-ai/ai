//! Typed errors returned by the MCP client and adapter.

use serde_json::Value;
use thiserror::Error;

use crate::McpAuthorizationChallenge;

#[derive(Debug, Error)]
/// Errors returned by MCP protocol, transport, and adapter operations.
pub enum Error {
    /// The server requires authorization before serving the request.
    #[error("[ai_mcp/error] authorization required")]
    AuthorizationRequired {
        /// Parsed Bearer challenge details.
        challenge: McpAuthorizationChallenge,
    },
    /// The server denied an authorized request.
    #[error("[ai_mcp/error] request forbidden")]
    Forbidden {
        /// Parsed Bearer challenge details.
        challenge: McpAuthorizationChallenge,
    },
    /// The active server-side MCP session expired.
    #[error("[ai_mcp/error] MCP session expired")]
    SessionExpired,
    /// The server selected a protocol version this client does not support.
    #[error(
        "[ai_mcp/error] unsupported protocol version `{server}` while requesting `{requested}`"
    )]
    UnsupportedProtocolVersion {
        /// Version sent in the initialization request.
        requested: String,
        /// Version selected by the server.
        server: String,
    },
    /// The server returned a JSON-RPC error response.
    #[error("[ai_mcp/error] `{method}` failed with JSON-RPC error {code}: {message}")]
    JsonRpc {
        /// MCP method that failed.
        method: String,
        /// JSON-RPC error code.
        code: i64,
        /// JSON-RPC error message.
        message: String,
        /// Optional structured error data.
        data: Option<Value>,
    },
    /// A response stream ended before the matching JSON-RPC response arrived.
    #[error("[ai_mcp/error] response for `{method}` was missing")]
    MissingResponse {
        /// MCP method awaiting a response.
        method: String,
    },
    /// The server returned another unsuccessful HTTP status.
    #[error("[ai_mcp/error] HTTP request failed with status {status}")]
    HttpStatus {
        /// HTTP status code.
        status: u16,
        /// Parsed or textual response body.
        body: Value,
    },
    /// An HTTP or SSE body exceeded the configured limit.
    #[error("[ai_mcp/error] response exceeded {limit_bytes} bytes")]
    ResponseTooLarge {
        /// Configured maximum response size.
        limit_bytes: usize,
    },
    /// A successful response did not match its typed MCP schema.
    #[error("[ai_mcp/error] failed to decode `{method}` response: {source}")]
    DeserializeResponse {
        /// MCP method being decoded.
        method: String,
        /// Underlying JSON decode failure.
        source: serde_json::Error,
    },
    /// The HTTP transport failed.
    #[error("[ai_mcp/error] transport failed: {message}")]
    Transport {
        /// Non-secret transport diagnostic.
        message: String,
    },
    /// The injected authentication hook failed.
    #[error("[ai_mcp/error] auth hook failed: {message}")]
    Auth {
        /// Non-secret authentication diagnostic.
        message: String,
    },
    /// The configured server key violates the naming contract.
    #[error("[ai_mcp/error] invalid server key `{server_key}`")]
    InvalidServerKey {
        /// Rejected server key.
        server_key: String,
    },
    /// The configured response cap is zero.
    #[error("[ai_mcp/error] response limit must be positive")]
    InvalidResponseLimit,
    /// A configured timeout is zero.
    #[error("[ai_mcp/error] request timeouts must be positive")]
    InvalidTimeout,
    /// The per-client JSON-RPC request ID space was exhausted.
    #[error("[ai_mcp/error] JSON-RPC request id space exhausted")]
    RequestIdExhausted,
    /// A response used an unsupported content type.
    #[error("[ai_mcp/error] unsupported response content type `{content_type}`")]
    UnsupportedContentType {
        /// Returned content type.
        content_type: String,
    },
}

impl Error {
    /// Builds a transport failure without exposing request secrets.
    #[track_caller]
    pub(crate) fn transport(source: &dyn std::fmt::Display) -> Self {
        Self::Transport {
            message: source.to_string(),
        }
    }

    /// Builds a response-deserialization error at the decode boundary.
    #[track_caller]
    pub(crate) fn deserialize(method: &str, source: serde_json::Error) -> Self {
        Self::DeserializeResponse {
            method: method.to_owned(),
            source,
        }
    }

    /// Builds an auth-hook failure at the authentication boundary.
    #[track_caller]
    pub(crate) fn auth(source: &dyn std::fmt::Display) -> Self {
        Self::Auth {
            message: source.to_string(),
        }
    }
}

/// Result alias for MCP operations.
pub type Result<T> = std::result::Result<T, Error>;
