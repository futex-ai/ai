//! Reqwest-backed buffered and SSE transport implementation.

use std::{sync::Arc, time::Duration};

use async_trait::async_trait;
use reqwest::header::CONTENT_TYPE;
use serde_json::Value;
use tokio::time::timeout;

use crate::{
    DynJsonHttpSseStream, Error, JsonHttpBody, JsonHttpMethod, JsonHttpMultipartField,
    JsonHttpRequest, JsonHttpResponse, JsonHttpTransport, Result,
    reqwest_sse::{ReqwestJsonHttpSseStream, SseTimer},
};

const CONNECT_TIMEOUT: Duration = Duration::from_secs(10);
const MAX_ERROR_BODY_BYTES: usize = 64 * 1024;

#[derive(Clone, Debug)]
enum ReqwestClientState {
    Ready(reqwest::Client),
    Failed(Arc<reqwest::Error>),
}

#[derive(Clone, Debug)]
pub(crate) struct ReqwestJsonHttpTransport {
    client: ReqwestClientState,
}

impl ReqwestJsonHttpTransport {
    pub(crate) fn new() -> Self {
        let client = reqwest::Client::builder()
            .connect_timeout(CONNECT_TIMEOUT)
            .build();
        Self {
            client: match client {
                Ok(client) => ReqwestClientState::Ready(client),
                Err(source) => ReqwestClientState::Failed(Arc::new(source)),
            },
        }
    }

    fn request_builder(&self, request: &JsonHttpRequest) -> Result<reqwest::RequestBuilder> {
        let client = match &self.client {
            ReqwestClientState::Ready(client) => client,
            ReqwestClientState::Failed(source) => {
                return Err(Error::ClientInitialization {
                    source: source.clone(),
                });
            }
        };
        let method = match request.method {
            JsonHttpMethod::Get => reqwest::Method::GET,
            JsonHttpMethod::Post => reqwest::Method::POST,
            JsonHttpMethod::Put => reqwest::Method::PUT,
            JsonHttpMethod::Delete => reqwest::Method::DELETE,
            JsonHttpMethod::Patch => reqwest::Method::PATCH,
        };
        let mut builder = client.request(method, &request.url);
        for (key, value) in &request.headers {
            builder = builder.header(key, value);
        }
        if let Some(body) = &request.body {
            builder = match body {
                JsonHttpBody::Json(body) => builder.json(body),
                JsonHttpBody::Multipart(multipart) => {
                    builder.multipart(reqwest_multipart_form(&multipart.fields)?)
                }
            };
        }
        Ok(builder)
    }

    async fn execute_buffered_request(
        &self,
        request: &JsonHttpRequest,
    ) -> Result<reqwest::Response> {
        let builder = self.request_builder(request)?.timeout(request.timeout);
        match builder.send().await {
            Ok(response) => Ok(response),
            Err(source) => Err(Error::ReqwestTransport { source }),
        }
    }
}

#[async_trait]
impl JsonHttpTransport for ReqwestJsonHttpTransport {
    async fn execute(&self, request: &JsonHttpRequest) -> Result<JsonHttpResponse<Value>> {
        let response = self.execute_buffered_request(request).await?;
        let status = response.status().as_u16();
        let text = match response.text().await {
            Ok(text) => text,
            Err(source) => return Err(Error::ReqwestTransport { source }),
        };
        let body = serde_json::from_str(&text).unwrap_or(Value::String(text));
        Ok(JsonHttpResponse { status, body })
    }

    async fn execute_bytes(&self, request: &JsonHttpRequest) -> Result<JsonHttpResponse<Vec<u8>>> {
        let response = self.execute_buffered_request(request).await?;
        let status = response.status().as_u16();
        let body = match response.bytes().await {
            Ok(body) => body.to_vec(),
            Err(source) => return Err(Error::ReqwestTransport { source }),
        };
        Ok(JsonHttpResponse { status, body })
    }

    async fn execute_sse(&self, request: &JsonHttpRequest) -> Result<DynJsonHttpSseStream> {
        let builder = self.request_builder(request)?;
        let timer = SseTimer::new(request.timeout, request.idle_timeout);
        let (wait, kind) = timer.next_wait();
        if wait.is_zero() {
            return Err(timer.error(kind, 0));
        }
        let mut response = match timeout(wait, builder.send()).await {
            Ok(Ok(response)) => response,
            Ok(Err(source)) => {
                return Err(Error::SseTransport {
                    events_received: 0,
                    source,
                });
            }
            Err(_) => return Err(timer.error(kind, 0)),
        };

        if !response.status().is_success() {
            let status = response.status().as_u16();
            let body = read_error_body(&mut response, &timer).await?;
            return Err(Error::HttpStatus { status, body });
        }
        let content_type = response
            .headers()
            .get(CONTENT_TYPE)
            .and_then(|value| value.to_str().ok())
            .map(ToOwned::to_owned);
        if !content_type.as_deref().is_some_and(is_event_stream) {
            return Err(Error::InvalidSseContentType { content_type });
        }

        Ok(Box::new(ReqwestJsonHttpSseStream::new(response, timer)))
    }
}

async fn read_error_body(response: &mut reqwest::Response, timer: &SseTimer) -> Result<Value> {
    let mut body = Vec::new();
    while body.len() < MAX_ERROR_BODY_BYTES {
        let (wait, kind) = timer.next_wait();
        if wait.is_zero() {
            return Err(timer.error(kind, 0));
        }
        match timeout(wait, response.chunk()).await {
            Ok(Ok(Some(bytes))) => {
                let remaining = MAX_ERROR_BODY_BYTES - body.len();
                body.extend_from_slice(&bytes[..bytes.len().min(remaining)]);
            }
            Ok(Ok(None)) => break,
            Ok(Err(source)) => {
                return Err(Error::SseTransport {
                    events_received: 0,
                    source,
                });
            }
            Err(_) => return Err(timer.error(kind, 0)),
        }
    }
    match serde_json::from_slice(&body) {
        Ok(body) => Ok(body),
        Err(_) => Ok(Value::String(String::from_utf8_lossy(&body).into_owned())),
    }
}

fn is_event_stream(content_type: &str) -> bool {
    content_type
        .split(';')
        .next()
        .is_some_and(|value| value.trim().eq_ignore_ascii_case("text/event-stream"))
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
                Err(source) => return Err(Error::ReqwestTransport { source }),
            };
        }
        form = form.part(field.name.clone(), part);
    }
    Ok(form)
}
