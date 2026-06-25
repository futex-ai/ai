//! Trait-backed client and reqwest transport implementations.

use std::sync::Arc;

use async_trait::async_trait;
use serde_json::Value;

use crate::{
    Error, JsonHttpBody, JsonHttpMethod, JsonHttpMultipartField, JsonHttpRequest,
    JsonHttpRequestBuilder, JsonHttpResponse, Result,
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
            inner: TransportBackedJsonHttpClient::new(Arc::new(ReqwestJsonHttpTransport {
                client: reqwest::Client::new(),
            })),
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

#[derive(Clone, Debug)]
struct ReqwestJsonHttpTransport {
    client: reqwest::Client,
}

#[async_trait]
impl JsonHttpTransport for ReqwestJsonHttpTransport {
    async fn execute(&self, request: &JsonHttpRequest) -> Result<JsonHttpResponse<Value>> {
        let method = match request.method {
            JsonHttpMethod::Get => reqwest::Method::GET,
            JsonHttpMethod::Post => reqwest::Method::POST,
            JsonHttpMethod::Put => reqwest::Method::PUT,
            JsonHttpMethod::Delete => reqwest::Method::DELETE,
            JsonHttpMethod::Patch => reqwest::Method::PATCH,
        };
        let mut builder = self
            .client
            .request(method, &request.url)
            .timeout(request.timeout);
        for (key, value) in &request.headers {
            builder = builder.header(key, value);
        }
        if let Some(body) = &request.body {
            match body {
                JsonHttpBody::Json(body) => {
                    builder = builder.json(body);
                }
                JsonHttpBody::Multipart(multipart) => {
                    builder = builder.multipart(reqwest_multipart_form(&multipart.fields)?);
                }
            }
        }

        let response = builder
            .send()
            .await
            .map_err(|source| crate::Error::transport(source.to_string()))?;
        let status = response.status().as_u16();
        let text = response
            .text()
            .await
            .map_err(|source| crate::Error::transport(source.to_string()))?;
        let body = serde_json::from_str(&text).unwrap_or(Value::String(text));
        Ok(JsonHttpResponse { status, body })
    }
}

fn reqwest_multipart_form(fields: &[JsonHttpMultipartField]) -> Result<reqwest::multipart::Form> {
    let mut form = reqwest::multipart::Form::new();
    for field in fields {
        let mut part = reqwest::multipart::Part::bytes(field.bytes.clone());
        if let Some(filename) = &field.filename {
            part = part.file_name(filename.clone());
        }
        if let Some(content_type) = &field.content_type {
            part = match part.mime_str(content_type) {
                Ok(part) => part,
                Err(source) => {
                    return Err(Error::transport(source.to_string()));
                }
            };
        }
        form = form.part(field.name.clone(), part);
    }
    Ok(form)
}
