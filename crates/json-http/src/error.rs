//! Error contract for JSON HTTP requests.

use std::{sync::Arc, time::Duration};

use serde_json::Value;
use thiserror::Error;

use crate::sse::JsonHttpSseDecodeError;

#[derive(Debug, Error)]
/// Errors returned by the JSON HTTP boundary.
pub enum Error {
    /// The configured reqwest client could not be constructed.
    #[error("[json_http/error] failed to initialize reqwest client: {source}")]
    ClientInitialization {
        /// Underlying reqwest client-construction failure.
        source: Arc<reqwest::Error>,
    },
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
    /// A reqwest failure occurred on a buffered HTTP request.
    #[error("[json_http/error] reqwest transport error: {source}")]
    ReqwestTransport {
        /// Underlying reqwest request or response-body failure.
        source: reqwest::Error,
    },
    /// Request auth hook failed while applying headers.
    #[error("[json_http/error] auth error: {message}")]
    Auth {
        /// Human-readable auth failure details.
        message: String,
    },
    /// The configured transport does not support SSE execution.
    #[error("[json_http/error] SSE execution is unsupported by this transport")]
    SseUnsupported,
    /// A streaming request opened with a non-success HTTP status.
    #[error("[json_http/error] SSE request returned HTTP {status}")]
    HttpStatus {
        /// HTTP status code returned by the server.
        status: u16,
        /// Bounded JSON or textual diagnostic body.
        body: Value,
    },
    /// A successful streaming response used a non-SSE media type.
    #[error("[json_http/error] expected text/event-stream, received {content_type:?}")]
    InvalidSseContentType {
        /// Raw content type when it was present and valid header text.
        content_type: Option<String>,
    },
    /// No complete SSE event arrived within the configured idle duration.
    #[error("[json_http/error] SSE stream was idle for {idle:?} after {events_received} events")]
    IdleTimeout {
        /// Configured maximum gap between events.
        idle: Duration,
        /// Number of events emitted before the timeout.
        events_received: u64,
    },
    /// The streaming request exceeded its overall deadline.
    #[error("[json_http/error] SSE stream exceeded {timeout:?} after {events_received} events")]
    DeadlineExceeded {
        /// Configured overall request duration.
        timeout: Duration,
        /// Number of events emitted before the deadline.
        events_received: u64,
    },
    /// A reqwest response-body read failed during SSE consumption.
    #[error("[json_http/error] SSE transport failed after {events_received} events: {source}")]
    SseTransport {
        /// Number of events emitted before the transport failure.
        events_received: u64,
        /// Underlying reqwest body error.
        source: reqwest::Error,
    },
    /// SSE framing failed during response consumption.
    #[error("[json_http/error] SSE decoding failed after {events_received} events: {source}")]
    SseDecode {
        /// Number of events emitted before the decoder failure.
        events_received: u64,
        /// Underlying pure decoder error.
        source: JsonHttpSseDecodeError,
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
