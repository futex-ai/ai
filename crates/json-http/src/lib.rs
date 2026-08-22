//! Typed JSON and binary HTTP client boundary with builder-style request assembly.

#![warn(unreachable_pub)]

mod auth;
mod client;
mod error;
mod request;
mod reqwest_sse;
mod reqwest_transport;
mod sse;

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
pub use request::{
    JsonHttpBody, JsonHttpMethod, JsonHttpMultipart, JsonHttpMultipartField, JsonHttpRequest,
    JsonHttpRequestBuilder, JsonHttpResponse,
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
