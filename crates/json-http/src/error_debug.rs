//! Structural HTTP error diagnostics omit credentials and provider payloads.

use std::fmt;

use crate::Error;

impl fmt::Debug for Error {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ResponseContext { headers, source } => formatter
                .debug_struct("ResponseContext")
                .field(
                    "headers",
                    &headers.keys().map(|name| name.as_str()).collect::<Vec<_>>(),
                )
                .field("source", source)
                .finish(),
            Self::HttpStatus {
                status,
                headers,
                body,
            } => formatter
                .debug_struct("HttpStatus")
                .field("status", status)
                .field(
                    "headers",
                    &headers.keys().map(|name| name.as_str()).collect::<Vec<_>>(),
                )
                .field(
                    "body_size",
                    &serde_json::to_vec(body)
                        .map(|bytes| bytes.len())
                        .unwrap_or_default(),
                )
                .finish(),
            Self::DeserializeResponse { body, .. } => formatter
                .debug_struct("DeserializeResponse")
                .field(
                    "body_size",
                    &serde_json::to_vec(body)
                        .map(|bytes| bytes.len())
                        .unwrap_or_default(),
                )
                .finish(),
            Self::ClientInitialization { .. } => formatter.write_str("ClientInitialization"),
            Self::SerializeRequest { .. } => formatter.write_str("SerializeRequest"),
            Self::Transport { .. } => formatter.write_str("Transport"),
            Self::ReqwestTransport { .. } => formatter.write_str("ReqwestTransport"),
            Self::Auth { .. } => formatter.write_str("Auth"),
            Self::SseUnsupported => formatter.write_str("SseUnsupported"),
            Self::InvalidSseContentType { .. } => formatter.write_str("InvalidSseContentType"),
            Self::IdleTimeout {
                idle,
                events_received,
            } => formatter
                .debug_struct("IdleTimeout")
                .field("idle", idle)
                .field("events_received", events_received)
                .finish(),
            Self::DeadlineExceeded {
                timeout,
                events_received,
            } => formatter
                .debug_struct("DeadlineExceeded")
                .field("timeout", timeout)
                .field("events_received", events_received)
                .finish(),
            Self::SseTransport {
                events_received, ..
            } => formatter
                .debug_struct("SseTransport")
                .field("events_received", events_received)
                .finish(),
            Self::SseDecode {
                events_received, ..
            } => formatter
                .debug_struct("SseDecode")
                .field("events_received", events_received)
                .finish(),
        }
    }
}
