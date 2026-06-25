//! Error contract for JSON HTTP requests.

use serde_json::Value;
use thiserror::Error;

#[derive(Debug, Error)]
/// Errors returned by the JSON HTTP boundary.
pub enum Error {
    /// Request body serialization failed before the request was sent.
    #[error("[json_http/error] failed to serialize request body: {source}")]
    SerializeRequest {
        /// Underlying serialization error.
        source: serde_json::Error,
    },
    /// Response body deserialization failed after a response was received.
    #[error("[json_http/error] failed to deserialize response body: {source}")]
    DeserializeResponse {
        /// Raw JSON body that failed to deserialize into the requested type.
        body: Value,
        /// Underlying deserialization error.
        source: serde_json::Error,
    },
    /// Transport-level request failure.
    #[error("[json_http/error] transport error: {message}")]
    Transport {
        /// Human-readable transport failure details.
        message: String,
    },
    /// Request auth hook failed while applying headers.
    #[error("[json_http/error] auth error: {message}")]
    Auth {
        /// Human-readable auth failure details.
        message: String,
    },
}

impl Error {
    /// Builds a transport error from a message.
    pub fn transport(message: impl Into<String>) -> Self {
        Self::Transport {
            message: message.into(),
        }
    }

    /// Builds an auth-hook error from a message.
    pub fn auth(message: impl Into<String>) -> Self {
        Self::Auth {
            message: message.into(),
        }
    }
}

/// Result alias for JSON HTTP operations.
pub type Result<T> = std::result::Result<T, Error>;
