//! Request builder and request/response DTOs for JSON HTTP calls.

use std::{
    collections::{BTreeMap, BTreeSet},
    time::Duration,
};

use serde::{Serialize, de::DeserializeOwned};
use serde_json::Value;

use crate::canonical::CanonicalSerializer;
use crate::{
    DynJsonHttpAuth, DynJsonHttpSseStream, DynJsonHttpTransport, Error, JsonHttpBody,
    JsonHttpMethod, JsonHttpMultipart, JsonHttpMultipartField, JsonHttpRequest, JsonHttpResponse,
    Result,
    request_headers::{credential_header, insert, same_origin},
};

const DEFAULT_TIMEOUT: Duration = Duration::from_secs(600);

#[derive(Clone)]
/// Builder used to assemble a JSON request before dispatch.
pub struct JsonHttpRequestBuilder {
    transport: DynJsonHttpTransport,
    request: JsonHttpRequest,
    auth_hooks: Vec<(DynJsonHttpAuth, Option<String>)>,
    default_headers: BTreeMap<String, String>,
    credential_origin: Option<String>,
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
                auth_headers: BTreeSet::new(),
                body: None,
                timeout: DEFAULT_TIMEOUT,
                idle_timeout: None,
            },
            auth_hooks: Vec::new(),
            default_headers: BTreeMap::new(),
            credential_origin: None,
        }
    }

    /// Adds or replaces a single request header.
    pub fn header(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        insert(&mut self.request.headers, key.into(), value.into());
        self
    }

    /// Adds or replaces a set of request headers.
    pub fn headers(mut self, headers: BTreeMap<String, String>) -> Self {
        for (name, value) in headers {
            insert(&mut self.request.headers, name, value);
        }
        self
    }

    /// Adds lower-priority headers after authentication, without replacing body content types.
    pub fn default_headers(mut self, headers: BTreeMap<String, String>) -> Self {
        for (name, value) in headers {
            insert(&mut self.default_headers, name, value);
        }
        self
    }

    /// Sends credential headers only to this scheme, host, and port.
    pub fn credentials_for_origin(mut self, origin: impl Into<String>) -> Self {
        self.credential_origin = Some(origin.into());
        self
    }

    /// Overrides the request transport timeout.
    pub fn timeout(mut self, timeout: Duration) -> Self {
        self.request.timeout = timeout;
        self
    }

    /// Sets the maximum gap between decoded SSE events.
    pub fn idle_timeout(mut self, idle_timeout: Duration) -> Self {
        self.request.idle_timeout = Some(idle_timeout);
        self
    }

    /// Registers an auth hook that will run before dispatch.
    pub fn auth(mut self, auth: DynJsonHttpAuth) -> Self {
        self.auth_hooks.push((auth, None));
        self
    }

    /// Applies an auth hook only when the request matches the supplied origin.
    pub fn auth_for_origin(mut self, auth: DynJsonHttpAuth, origin: impl Into<String>) -> Self {
        self.auth_hooks.push((auth, Some(origin.into())));
        self
    }

    /// Serializes a typed JSON request body into the builder.
    pub fn json<T>(mut self, body: T) -> Result<Self>
    where
        T: Serialize,
    {
        let body = match body.serialize(CanonicalSerializer) {
            Ok(body) => body,
            Err(source) => return Err(Error::SerializeRequest { source }),
        };
        self.request.body = Some(JsonHttpBody::Json(body));
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
        match transport.execute(&request).await {
            Ok(response) => Ok(response),
            Err(source) => Err(source.sanitize_urls()),
        }
    }

    /// Executes the request and returns the raw response bytes.
    pub async fn send_bytes(self) -> Result<JsonHttpResponse<Vec<u8>>> {
        let transport = self.transport.clone();
        let request = self.build_request().await?;
        match transport.execute_bytes(&request).await {
            Ok(response) => Ok(response),
            Err(source) => Err(source.sanitize_urls()),
        }
    }

    /// Opens a decoded Server-Sent Events response stream.
    pub async fn send_sse(self) -> Result<DynJsonHttpSseStream> {
        let transport = self.transport.clone();
        let request = self.build_request().await?;
        match transport.execute_sse(&request).await {
            Ok(response) => Ok(response),
            Err(source) => Err(source.sanitize_urls()),
        }
    }

    /// Executes the request and deserializes the response body into a typed DTO.
    pub async fn send<T>(self) -> Result<JsonHttpResponse<T>>
    where
        T: DeserializeOwned,
    {
        let response = self.send_value().await?;
        let body = match serde_json::from_value(response.body.clone()) {
            Ok(body) => body,
            Err(source) => {
                return Err(Error::DeserializeResponse {
                    body: response.body,
                    source,
                }
                .with_response_headers(response.headers));
            }
        };
        Ok(JsonHttpResponse {
            status: response.status,
            headers: response.headers,
            body,
        })
    }

    async fn build_request(mut self) -> Result<JsonHttpRequest> {
        let adapter_headers = self.request.headers.clone();
        let credentials_allowed = self
            .credential_origin
            .as_deref()
            .is_none_or(|origin| same_origin(origin, &self.request.url));
        for (auth, origin) in self.auth_hooks {
            if !credentials_allowed
                || origin.is_some_and(|origin| !same_origin(&origin, &self.request.url))
            {
                continue;
            }
            let before = self.request.headers.clone();
            auth.apply_headers(&mut self.request.headers).await?;
            let changed = self
                .request
                .headers
                .iter()
                .filter(|(name, value)| before.get(*name) != Some(*value))
                .map(|(name, value)| (name.clone(), value.clone()))
                .collect::<Vec<_>>();
            for (name, value) in changed {
                self.request.auth_headers.insert(name.to_ascii_lowercase());
                insert(&mut self.request.headers, name, value);
            }
        }
        for (name, value) in self.default_headers {
            if !credentials_allowed {
                continue;
            }
            if self.request.body.is_some() && name.eq_ignore_ascii_case("content-type") {
                continue;
            }
            if self
                .request
                .headers
                .keys()
                .any(|key| key.eq_ignore_ascii_case(&name))
            {
                continue;
            }
            insert(&mut self.request.headers, name, value);
        }
        for (name, value) in &self.request.headers {
            if credential_header(name)
                || !adapter_headers
                    .iter()
                    .any(|(key, original)| key.eq_ignore_ascii_case(name) && original == value)
            {
                self.request.auth_headers.insert(name.to_ascii_lowercase());
            }
        }
        if !credentials_allowed {
            self.request.headers.retain(|name, _| {
                !self
                    .request
                    .auth_headers
                    .contains(&name.to_ascii_lowercase())
            });
        }
        Ok(self.request)
    }
}
