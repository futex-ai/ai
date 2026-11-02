//! Secret-free classification and reconstruction of transport failures.

use std::time::Duration;

use reqwest::IntoUrl;
use serde::{Deserialize, Serialize};

use crate::{Error, JsonHttpHeaders, JsonHttpSseDecoder};

/// Persisted failure facts, without diagnostic strings from a real request.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(tag = "class", rename_all = "snake_case")]
pub enum RecordedFailure {
    /// Connection or response-body failure.
    Transport,
    /// Maximum interval between events elapsed.
    IdleTimeout {
        /// Configured interval.
        idle: Duration,
    },
    /// Overall stream deadline elapsed.
    DeadlineExceeded {
        /// Configured overall duration.
        timeout: Duration,
    },
    /// Invalid SSE framing or encoding.
    Decode,
    /// The transport does not implement SSE.
    Unsupported,
    /// Authentication failed before dispatch.
    Auth,
    /// An SSE opening returned another content type.
    InvalidContentType,
    /// A call has started but has not returned a response.
    Pending,
}

impl RecordedFailure {
    pub(super) fn capture(error: &Error) -> Self {
        match error.cause() {
            Error::IdleTimeout { idle, .. } => Self::IdleTimeout { idle: *idle },
            Error::DeadlineExceeded { timeout, .. } => Self::DeadlineExceeded { timeout: *timeout },
            Error::SseDecode { .. } => Self::Decode,
            Error::SseUnsupported => Self::Unsupported,
            Error::Auth { .. } => Self::Auth,
            Error::InvalidSseContentType { .. } => Self::InvalidContentType,
            _ => Self::Transport,
        }
    }

    pub(super) fn error_with_headers(
        &self,
        events_received: u64,
        headers: Option<JsonHttpHeaders>,
    ) -> Error {
        let error = self.error(events_received);
        match headers {
            Some(headers) => error.with_response_headers(headers),
            None => error,
        }
    }

    pub(super) fn error(&self, events_received: u64) -> Error {
        match self {
            Self::IdleTimeout { idle } => Error::IdleTimeout {
                idle: *idle,
                events_received,
            },
            Self::DeadlineExceeded { timeout } => Error::DeadlineExceeded {
                timeout: *timeout,
                events_received,
            },
            Self::Decode => {
                let mut decoder = JsonHttpSseDecoder::new();
                decoder.push(&[b'd', b'a', b't', b'a', b':', 0xff, b'\n', b'\n']);
                match decoder.next_event(false) {
                    Err(source) => Error::SseDecode {
                        source,
                        events_received,
                    },
                    Ok(_) => Error::transport("recorded decode failure"),
                }
            }
            Self::Unsupported => Error::SseUnsupported,
            Self::Auth => Error::auth("recorded auth failure"),
            Self::InvalidContentType => Error::InvalidSseContentType { content_type: None },
            Self::Pending => Error::transport("recording is incomplete"),
            Self::Transport => transport_failure("recorded-stream-failure", events_received),
        }
    }
}

/// Creates the reqwest source required by the existing SSE error without IO.
fn transport_failure(url: impl IntoUrl, events_received: u64) -> Error {
    match url.into_url() {
        Err(source) => Error::SseTransport {
            source,
            events_received,
        },
        Ok(_) => Error::transport("recorded transport failure"),
    }
}
