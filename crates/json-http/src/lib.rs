//! Typed JSON and binary HTTP client boundary with builder-style request assembly.

#![warn(unreachable_pub)]

mod auth;
mod canonical;
mod client;
mod error;
mod headers;
mod request;
mod request_headers;
mod request_types;
mod reqwest_redirect;
mod reqwest_sse;
mod reqwest_transport;
mod sse;

#[cfg(any(test, feature = "test-support"))]
pub mod test_support;

#[cfg(any(test, feature = "test-support"))]
pub use test_support::{
    MemoryRecordingLog, MultipartSummary, RecordedBody, RecordedEvent, RecordedFailure,
    RecordedMethod, RecordedRequest, RecordedResponse, Recording, RecordingLog, RecordingTransport,
    Redactor, ReplayControl, ReplayError, ReplayTransport, StreamEnd,
};

#[cfg(any(test, doctest, feature = "test-support"))]
pub use auth::JsonHttpAuthMock;
pub use auth::{DynJsonHttpAuth, JsonHttpAuth, StaticHeaderAuth};
#[cfg(any(test, doctest, feature = "test-support"))]
pub use client::JsonHttpTransportMock;
pub use client::{
    DynJsonHttpClient, DynJsonHttpTransport, JsonHttpClient, JsonHttpTransport,
    ReqwestJsonHttpClient, TransportBackedJsonHttpClient,
};
pub use error::{Error, Result};
pub use headers::JsonHttpHeaders;
pub use request::JsonHttpRequestBuilder;
pub use request_types::{
    JsonHttpBody, JsonHttpMethod, JsonHttpMultipart, JsonHttpMultipartField, JsonHttpRequest,
    JsonHttpResponse,
};
#[cfg(any(test, doctest, feature = "test-support"))]
pub use sse::JsonHttpSseStreamMock;
pub use sse::{
    DynJsonHttpSseStream, JsonHttpSseDecodeError, JsonHttpSseDecoder, JsonHttpSseEvent,
    JsonHttpSseStream,
};

#[cfg(test)]
#[path = "_tests_/mod.rs"]
mod tests;
