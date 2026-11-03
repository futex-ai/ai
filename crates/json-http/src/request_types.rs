//! Serialized request and response types at the transport boundary.

use std::{
    collections::{BTreeMap, BTreeSet},
    fmt,
    time::Duration,
};

use crate::JsonHttpHeaders;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
/// Supported JSON HTTP verbs.
pub enum JsonHttpMethod {
    /// `GET`
    Get,
    /// `POST`
    Post,
    /// `PUT`
    Put,
    /// `DELETE`
    Delete,
    /// `PATCH`
    Patch,
}

#[derive(Clone, Debug, PartialEq)]
/// Serialized request body passed to the transport.
pub enum JsonHttpBody {
    /// JSON request body.
    Json(Vec<u8>),
    /// Multipart form request body.
    Multipart(JsonHttpMultipart),
}

impl JsonHttpBody {
    /// Returns encoded JSON bytes when this body is JSON.
    pub fn as_json_bytes(&self) -> Option<&[u8]> {
        match self {
            Self::Json(bytes) => Some(bytes),
            Self::Multipart(_) => None,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
/// Multipart request body with byte fields.
pub struct JsonHttpMultipart {
    /// Multipart fields in insertion order.
    pub fields: Vec<JsonHttpMultipartField>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
/// One multipart byte field.
pub struct JsonHttpMultipartField {
    /// Form field name.
    pub name: String,
    /// Optional uploaded filename.
    pub filename: Option<String>,
    /// Optional field content type.
    pub content_type: Option<String>,
    /// Raw field bytes.
    pub bytes: Vec<u8>,
}

impl JsonHttpMultipartField {
    /// Builds a multipart byte field.
    pub fn bytes(name: impl Into<String>, bytes: Vec<u8>) -> Self {
        Self {
            name: name.into(),
            filename: None,
            content_type: None,
            bytes,
        }
    }

    /// Sets the multipart filename.
    pub fn filename(mut self, filename: impl Into<String>) -> Self {
        self.filename = Some(filename.into());
        self
    }

    /// Sets the multipart content type.
    pub fn content_type(mut self, content_type: impl Into<String>) -> Self {
        self.content_type = Some(content_type.into());
        self
    }
}

#[derive(Clone, PartialEq)]
/// Serialized JSON HTTP request passed to the transport.
pub struct JsonHttpRequest {
    /// HTTP method to use for the request.
    pub method: JsonHttpMethod,
    /// Fully-qualified request URL.
    pub url: String,
    /// Request headers after auth hooks are applied.
    pub headers: BTreeMap<String, String>,
    /// Case-insensitive names of headers set by authentication hooks.
    pub auth_headers: BTreeSet<String>,
    /// Optional request body.
    pub body: Option<JsonHttpBody>,
    /// Per-request transport timeout.
    pub timeout: Duration,
    /// Optional maximum gap between decoded events on the SSE path.
    pub idle_timeout: Option<Duration>,
}

#[derive(Clone, PartialEq)]
/// Typed JSON HTTP response returned from the client.
pub struct JsonHttpResponse<T> {
    /// HTTP status code.
    pub status: u16,
    /// Response headers received before body consumption.
    pub headers: JsonHttpHeaders,
    /// Parsed JSON body.
    pub body: T,
}

impl<T> fmt::Debug for JsonHttpResponse<T> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("JsonHttpResponse")
            .field("status", &self.status)
            .field("headers", &self.headers)
            .field("body", &"[redacted]")
            .finish()
    }
}

impl fmt::Debug for JsonHttpRequest {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("JsonHttpRequest")
            .field("method", &self.method)
            .field("url", &"[redacted]")
            .field("headers", &self.headers.keys().collect::<Vec<_>>())
            .field("auth_headers", &self.auth_headers)
            .field("body", &self.body.as_ref().map(|_| "[redacted]"))
            .field("timeout", &self.timeout)
            .field("idle_timeout", &self.idle_timeout)
            .finish()
    }
}
