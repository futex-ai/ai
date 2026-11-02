//! Transparent recording around an injected transport and its lazy SSE streams.

use std::sync::Arc;

use async_trait::async_trait;
use base64::{Engine, prelude::BASE64_STANDARD};
use serde_json::Value;

use crate::{
    DynJsonHttpSseStream, DynJsonHttpTransport, Error, JsonHttpHeaders, JsonHttpRequest,
    JsonHttpResponse, JsonHttpSseEvent, JsonHttpSseStream, JsonHttpTransport, Result,
    reqwest_transport::ReqwestJsonHttpTransport,
};

use super::{RecordedEvent, RecordedFailure, RecordedResponse, RecordingLog, Redactor, StreamEnd};

/// Forwards requests unchanged and records only sanitized copies.
pub struct RecordingTransport {
    inner: DynJsonHttpTransport,
    log: Arc<dyn RecordingLog + Send + Sync>,
}

impl RecordingTransport {
    /// Wraps any transport with a shared recording log.
    pub fn new(inner: DynJsonHttpTransport, log: Arc<dyn RecordingLog + Send + Sync>) -> Self {
        Self { inner, log }
    }

    /// Composition helper for the production reqwest transport.
    pub fn reqwest(log: Arc<dyn RecordingLog + Send + Sync>) -> Self {
        Self::new(Arc::new(ReqwestJsonHttpTransport::new()), log)
    }

    fn start(&self, request: &JsonHttpRequest) -> (usize, Redactor) {
        let redactor = Redactor::for_request(request);
        (self.log.begin(redactor.request(request)), redactor)
    }

    fn failed(&self, index: usize, error: &Error, redactor: &Redactor) {
        let response = match error {
            Error::HttpStatus {
                status,
                headers,
                body,
            } => RecordedResponse::Json {
                headers: redactor.headers(headers),
                status: *status,
                body: redactor.value(body),
            },
            error => RecordedResponse::Failure {
                failure: RecordedFailure::capture(error),
            },
        };
        self.log.response(index, response);
    }
}

#[async_trait]
impl JsonHttpTransport for RecordingTransport {
    async fn execute(&self, request: &JsonHttpRequest) -> Result<JsonHttpResponse<Value>> {
        let (index, redactor) = self.start(request);
        let result = self.inner.execute(request).await;
        match &result {
            Ok(response) => self.log.response(
                index,
                RecordedResponse::Json {
                    headers: redactor.headers(&response.headers),
                    status: response.status,
                    body: redactor.value(&response.body),
                },
            ),
            Err(error) => self.failed(index, error, &redactor),
        }
        result
    }

    async fn execute_bytes(&self, request: &JsonHttpRequest) -> Result<JsonHttpResponse<Vec<u8>>> {
        let (index, redactor) = self.start(request);
        let result = self.inner.execute_bytes(request).await;
        match &result {
            Ok(response) => self.log.response(
                index,
                RecordedResponse::Bytes {
                    headers: redactor.headers(&response.headers),
                    status: response.status,
                    body_base64: BASE64_STANDARD.encode(redactor.bytes(&response.body)),
                },
            ),
            Err(error) => self.failed(index, error, &redactor),
        }
        result
    }

    async fn execute_sse(&self, request: &JsonHttpRequest) -> Result<DynJsonHttpSseStream> {
        let (index, redactor) = self.start(request);
        match self.inner.execute_sse(request).await {
            Ok(inner) => {
                self.log.response(
                    index,
                    RecordedResponse::Sse {
                        headers: redactor.headers(inner.headers()),
                        status: 200,
                        events: Vec::new(),
                        end: StreamEnd::Done,
                    },
                );
                Ok(Box::new(RecordingStream {
                    inner,
                    log: self.log.clone(),
                    index,
                    redactor,
                    events_received: 0,
                }))
            }
            Err(error) => {
                self.failed(index, &error, &redactor);
                Err(error)
            }
        }
    }
}

struct RecordingStream {
    inner: DynJsonHttpSseStream,
    log: Arc<dyn RecordingLog + Send + Sync>,
    index: usize,
    redactor: Redactor,
    events_received: u64,
}

#[async_trait]
impl JsonHttpSseStream for RecordingStream {
    fn headers(&self) -> &JsonHttpHeaders {
        self.inner.headers()
    }

    async fn next(&mut self) -> Result<Option<JsonHttpSseEvent>> {
        let result = self.inner.next().await;
        match &result {
            Ok(Some(event)) => {
                self.events_received += 1;
                self.log.event(
                    self.index,
                    RecordedEvent {
                        event: event.event.as_ref().map(|name| self.redactor.text(name)),
                        data: self.redactor.event_data(&event.data),
                    },
                );
            }
            Err(error) => self.log.end(
                self.index,
                StreamEnd::Failure {
                    after_events: self.events_received,
                    failure: RecordedFailure::capture(error),
                },
            ),
            Ok(None) => {}
        }
        result
    }
}
