//! Trait-backed client and reqwest transport implementations.

use std::sync::Arc;

use async_trait::async_trait;
use serde_json::Value;

use crate::{
    DynJsonHttpSseStream, Error, JsonHttpMethod, JsonHttpRequest, JsonHttpRequestBuilder,
    JsonHttpResponse, Result, reqwest_transport::ReqwestJsonHttpTransport,
};

/// Shared dynamic JSON HTTP client alias.
pub type DynJsonHttpClient = Arc<dyn JsonHttpClient>;

/// Shared dynamic JSON HTTP transport alias.
pub type DynJsonHttpTransport = Arc<dyn JsonHttpTransport>;

#[cfg_attr(
    any(test, doctest, feature = "test-support"),
    unimock::unimock(api = JsonHttpTransportMock)
)]
#[async_trait]
/// Low-level transport boundary used by the builder-style client.
pub trait JsonHttpTransport: Send + Sync {
    /// Executes one serialized JSON request and returns the raw JSON response body.
    async fn execute(&self, request: &JsonHttpRequest) -> Result<JsonHttpResponse<Value>>;

    /// Executes one request and returns the raw response bytes.
    async fn execute_bytes(&self, request: &JsonHttpRequest) -> Result<JsonHttpResponse<Vec<u8>>>;

    /// Opens one Server-Sent Events response stream.
    async fn execute_sse(&self, _request: &JsonHttpRequest) -> Result<DynJsonHttpSseStream> {
        Err(Error::SseUnsupported)
    }
}

/// Builder-oriented JSON HTTP client boundary.
pub trait JsonHttpClient: Send + Sync {
    /// Builds a `GET` request.
    fn get(&self, url: &str) -> JsonHttpRequestBuilder;

    /// Builds a `POST` request.
    fn post(&self, url: &str) -> JsonHttpRequestBuilder;

    /// Builds a `PUT` request.
    fn put(&self, url: &str) -> JsonHttpRequestBuilder;

    /// Builds a `DELETE` request.
    fn delete(&self, url: &str) -> JsonHttpRequestBuilder;

    /// Builds a `PATCH` request.
    fn patch(&self, url: &str) -> JsonHttpRequestBuilder;
}

#[derive(Clone)]
/// Generic JSON HTTP client backed by an injected transport.
pub struct TransportBackedJsonHttpClient {
    transport: DynJsonHttpTransport,
}

impl TransportBackedJsonHttpClient {
    /// Builds a client from the provided transport implementation.
    pub fn new(transport: DynJsonHttpTransport) -> Self {
        Self { transport }
    }

    fn builder(&self, method: JsonHttpMethod, url: &str) -> JsonHttpRequestBuilder {
        JsonHttpRequestBuilder::new(self.transport.clone(), method, url)
    }
}

impl JsonHttpClient for TransportBackedJsonHttpClient {
    fn get(&self, url: &str) -> JsonHttpRequestBuilder {
        self.builder(JsonHttpMethod::Get, url)
    }

    fn post(&self, url: &str) -> JsonHttpRequestBuilder {
        self.builder(JsonHttpMethod::Post, url)
    }

    fn put(&self, url: &str) -> JsonHttpRequestBuilder {
        self.builder(JsonHttpMethod::Put, url)
    }

    fn delete(&self, url: &str) -> JsonHttpRequestBuilder {
        self.builder(JsonHttpMethod::Delete, url)
    }

    fn patch(&self, url: &str) -> JsonHttpRequestBuilder {
        self.builder(JsonHttpMethod::Patch, url)
    }
}

#[derive(Clone)]
/// Reqwest-backed JSON HTTP client for production use.
pub struct ReqwestJsonHttpClient {
    inner: TransportBackedJsonHttpClient,
}

impl ReqwestJsonHttpClient {
    /// Builds a reqwest-backed JSON HTTP client with the default timeout.
    pub fn new() -> Self {
        Self {
            inner: TransportBackedJsonHttpClient::new(Arc::new(ReqwestJsonHttpTransport::new())),
        }
    }
}

impl Default for ReqwestJsonHttpClient {
    fn default() -> Self {
        Self::new()
    }
}

impl JsonHttpClient for ReqwestJsonHttpClient {
    fn get(&self, url: &str) -> JsonHttpRequestBuilder {
        self.inner.get(url)
    }

    fn post(&self, url: &str) -> JsonHttpRequestBuilder {
        self.inner.post(url)
    }

    fn put(&self, url: &str) -> JsonHttpRequestBuilder {
        self.inner.put(url)
    }

    fn delete(&self, url: &str) -> JsonHttpRequestBuilder {
        self.inner.delete(url)
    }

    fn patch(&self, url: &str) -> JsonHttpRequestBuilder {
        self.inner.patch(url)
    }
}
