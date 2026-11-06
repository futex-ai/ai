//! Ordered, credential-free playback through the real transport boundary.

use std::{collections::VecDeque, sync::Mutex};

use async_trait::async_trait;
use base64::{Engine, prelude::BASE64_STANDARD};
use serde_json::Value;

use crate::{
    DynJsonHttpSseStream, Error, JsonHttpHeaders, JsonHttpRequest, JsonHttpResponse,
    JsonHttpSseEvent, JsonHttpSseStream, JsonHttpTransport, Result,
};

use super::{RecordedResponse, Recording, Redactor, StreamEnd};

/// Replay validation errors translated to the existing HTTP transport error contract.
#[derive(Debug, thiserror::Error)]
pub enum ReplayError {
    /// A request arrived after every exchange was consumed.
    #[error("[json_http/replay] no recording remains")]
    Exhausted,
    /// The next exchange belongs to a different method or URL.
    #[error("[json_http/replay] request method or URL does not match the next recording")]
    RequestMismatch,
    /// The recorded response belongs to another transport method.
    #[error("[json_http/replay] response kind does not match the transport call")]
    ResponseMismatch,
    /// The binary recording contains invalid base64.
    #[error("[json_http/replay] invalid base64 response: {source}")]
    InvalidBase64 {
        /// Decode error without response contents.
        source: base64::DecodeError,
    },
}

impl ReplayError {
    fn http(self) -> Error {
        Error::transport(self.to_string())
    }
}

/// A transport that consumes recordings in their original call order.
pub struct ReplayTransport {
    recordings: Mutex<VecDeque<Recording>>,
}

/// Typed replay cursor control, separate from the existing HTTP error contract.
pub trait ReplayControl: Send + Sync {
    /// Reports exchanges that were not used by the adapter.
    fn remaining(&self) -> usize;
    /// Takes the next response without advancing on a request mismatch.
    fn next_response(
        &self,
        request: &JsonHttpRequest,
    ) -> std::result::Result<RecordedResponse, ReplayError>;
}

impl ReplayTransport {
    /// Creates an independent replay cursor.
    pub fn new(recordings: Vec<Recording>) -> Self {
        Self {
            recordings: Mutex::new(recordings.into()),
        }
    }

    fn response(&self, request: &JsonHttpRequest) -> Result<RecordedResponse> {
        match self.next_response(request) {
            Ok(response) => Ok(response),
            Err(error) => Err(error.http()),
        }
    }
}

impl ReplayControl for ReplayTransport {
    fn remaining(&self) -> usize {
        self.recordings
            .lock()
            .unwrap_or_else(|poison| poison.into_inner())
            .len()
    }

    fn next_response(
        &self,
        request: &JsonHttpRequest,
    ) -> std::result::Result<RecordedResponse, ReplayError> {
        let mut recordings = self
            .recordings
            .lock()
            .unwrap_or_else(|poison| poison.into_inner());
        let Some(next) = recordings.front() else {
            return Err(ReplayError::Exhausted);
        };
        let request = Redactor::for_request(request).request(request);
        if next.request.method != request.method || next.request.url != request.url {
            return Err(ReplayError::RequestMismatch);
        }
        match recordings.pop_front() {
            Some(recording) => Ok(recording.response),
            None => Err(ReplayError::Exhausted),
        }
    }
}

#[async_trait]
impl JsonHttpTransport for ReplayTransport {
    async fn execute(&self, request: &JsonHttpRequest) -> Result<JsonHttpResponse<Value>> {
        match self.response(request)? {
            RecordedResponse::Json {
                status,
                headers,
                body,
            } => Ok(JsonHttpResponse {
                headers,
                status,
                body,
            }),
            RecordedResponse::Failure { failure, headers } => {
                Err(failure.error_with_headers(0, headers))
            }
            _ => Err(ReplayError::ResponseMismatch.http()),
        }
    }

    async fn execute_bytes(&self, request: &JsonHttpRequest) -> Result<JsonHttpResponse<Vec<u8>>> {
        match self.response(request)? {
            RecordedResponse::Bytes {
                status,
                body_base64,
                headers,
            } => match BASE64_STANDARD.decode(body_base64) {
                Ok(body) => Ok(JsonHttpResponse {
                    headers,
                    status,
                    body,
                }),
                Err(source) => Err(ReplayError::InvalidBase64 { source }.http()),
            },
            RecordedResponse::Failure { failure, headers } => {
                Err(failure.error_with_headers(0, headers))
            }
            _ => Err(ReplayError::ResponseMismatch.http()),
        }
    }

    async fn execute_sse(&self, request: &JsonHttpRequest) -> Result<DynJsonHttpSseStream> {
        match self.response(request)? {
            RecordedResponse::Sse {
                status,
                events,
                end,
                headers,
            } if (200..300).contains(&status) => Ok(Box::new(ReplayStream {
                events: events.into(),
                end: Some(end),
                events_received: 0,
                headers,
            })),
            RecordedResponse::Json {
                status,
                headers,
                body,
            } if status >= 400 => Err(Error::HttpStatus {
                status,
                headers: Box::new(headers),
                body,
            }),
            RecordedResponse::Failure { failure, headers } => {
                Err(failure.error_with_headers(0, headers))
            }
            _ => Err(ReplayError::ResponseMismatch.http()),
        }
    }
}

struct ReplayStream {
    events: VecDeque<super::RecordedEvent>,
    end: Option<StreamEnd>,
    events_received: u64,
    headers: JsonHttpHeaders,
}

#[async_trait]
impl JsonHttpSseStream for ReplayStream {
    fn headers(&self) -> &JsonHttpHeaders {
        &self.headers
    }

    async fn next(&mut self) -> Result<Option<JsonHttpSseEvent>> {
        if let Some(StreamEnd::Failure { after_events, .. }) = &self.end
            && self.events_received >= *after_events
            && let Some(StreamEnd::Failure { failure, .. }) = self.end.take()
        {
            self.events.clear();
            return Err(failure.error(self.events_received));
        }
        if let Some(event) = self.events.pop_front() {
            self.events_received += 1;
            return Ok(Some(JsonHttpSseEvent {
                event: event.event,
                data: event.data,
            }));
        }
        self.end = None;
        Ok(None)
    }
}
