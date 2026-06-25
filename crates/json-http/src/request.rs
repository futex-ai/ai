//! Request builder and request/response DTOs for JSON HTTP calls.

use std::{collections::BTreeMap, ops::Index, time::Duration};

use serde::{Serialize, de::DeserializeOwned};
use serde_json::Value;

use crate::{DynJsonHttpAuth, DynJsonHttpTransport, Error, Result};

const DEFAULT_TIMEOUT: Duration = Duration::from_secs(60);

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
    Json(Value),
    /// Multipart form request body.
    Multipart(JsonHttpMultipart),
}

static JSON_NULL: Value = Value::Null;

impl JsonHttpBody {
    /// Returns the JSON value when this body is JSON.
    pub fn as_json(&self) -> Option<&Value> {
        match self {
            Self::Json(value) => Some(value),
            Self::Multipart(_) => None,
        }
    }

    /// Returns a field from the JSON object body.
    pub fn get(&self, key: &str) -> Option<&Value> {
        self.as_json().and_then(|value| value.get(key))
    }
}

impl Index<&str> for JsonHttpBody {
    type Output = Value;

    fn index(&self, index: &str) -> &Self::Output {
        match self {
            Self::Json(value) => &value[index],
            Self::Multipart(_) => &JSON_NULL,
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

#[derive(Clone, Debug, PartialEq)]
/// Serialized JSON HTTP request passed to the transport.
pub struct JsonHttpRequest {
    /// HTTP method to use for the request.
    pub method: JsonHttpMethod,
    /// Fully-qualified request URL.
    pub url: String,
    /// Request headers after auth hooks are applied.
    pub headers: BTreeMap<String, String>,
    /// Optional request body.
    pub body: Option<JsonHttpBody>,
    /// Per-request transport timeout.
    pub timeout: Duration,
}

#[derive(Clone, Debug, PartialEq)]
/// Typed JSON HTTP response returned from the client.
pub struct JsonHttpResponse<T> {
    /// HTTP status code.
    pub status: u16,
    /// Parsed JSON body.
    pub body: T,
}

#[derive(Clone)]
/// Builder used to assemble a JSON request before dispatch.
pub struct JsonHttpRequestBuilder {
    transport: DynJsonHttpTransport,
    request: JsonHttpRequest,
    auth_hooks: Vec<DynJsonHttpAuth>,
}

impl JsonHttpRequestBuilder {
    /// Creates a builder for one method and URL over the provided transport.
    pub fn new(
        transport: DynJsonHttpTransport,
        method: JsonHttpMethod,
        url: impl Into<String>,
    ) -> Self {
        Self {
            transport,
            request: JsonHttpRequest {
                method,
                url: url.into(),
                headers: BTreeMap::new(),
                body: None,
                timeout: DEFAULT_TIMEOUT,
            },
            auth_hooks: Vec::new(),
        }
    }

    /// Adds or replaces a single request header.
    pub fn header(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.request.headers.insert(key.into(), value.into());
        self
    }

    /// Adds or replaces a set of request headers.
    pub fn headers(mut self, headers: BTreeMap<String, String>) -> Self {
        self.request.headers.extend(headers);
        self
    }

    /// Overrides the request transport timeout.
    pub fn timeout(mut self, timeout: Duration) -> Self {
        self.request.timeout = timeout;
        self
    }

    /// Registers an auth hook that will run before dispatch.
    pub fn auth(mut self, auth: DynJsonHttpAuth) -> Self {
        self.auth_hooks.push(auth);
        self
    }

    /// Serializes a typed JSON request body into the builder.
    pub fn json<T>(mut self, body: T) -> Result<Self>
    where
        T: Serialize,
    {
        self.request.body = Some(JsonHttpBody::Json(
            serde_json::to_value(body).map_err(|source| Error::SerializeRequest { source })?,
        ));
        Ok(self)
    }

    /// Attaches a multipart request body to the builder.
    pub fn multipart(mut self, fields: Vec<JsonHttpMultipartField>) -> Self {
        self.request.body = Some(JsonHttpBody::Multipart(JsonHttpMultipart { fields }));
        self
    }

    /// Executes the request and returns the raw JSON value body.
    pub async fn send_value(self) -> Result<JsonHttpResponse<Value>> {
        let transport = self.transport.clone();
        let request = self.build_request().await?;
        transport.execute(&request).await
    }

    /// Executes the request and deserializes the response body into a typed DTO.
    pub async fn send<T>(self) -> Result<JsonHttpResponse<T>>
    where
        T: DeserializeOwned,
    {
        let response = self.send_value().await?;
        let body = serde_json::from_value(response.body.clone()).map_err(|source| {
            Error::DeserializeResponse {
                body: response.body,
                source,
            }
        })?;
        Ok(JsonHttpResponse {
            status: response.status,
            body,
        })
    }

    async fn build_request(mut self) -> Result<JsonHttpRequest> {
        for auth in self.auth_hooks {
            auth.apply_headers(&mut self.request.headers).await?;
        }
        Ok(self.request)
    }
}
