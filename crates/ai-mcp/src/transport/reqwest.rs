//! Reqwest implementation of the MCP streamable HTTP transport.

use std::{collections::BTreeMap, time::Duration};

use async_trait::async_trait;
use futures_util::StreamExt;
use reqwest::{Method, Response, redirect::Policy};
use serde_json::Value;

use crate::{Error, Result, authorization::is_authorization_status};

use super::{
    McpHttpPayload, McpHttpResponse, McpHttpTransport,
    content_type::{EVENT_STREAM, matches},
    delete_status::is_tolerated_delete_status,
    session_status::{has_session_header, is_expired_session_status},
    sse::ReqwestEventStream,
};

#[derive(Clone, Debug)]
/// Production MCP HTTP transport backed by reqwest and rustls.
pub struct ReqwestMcpHttpTransport {
    client: reqwest::Client,
}

impl ReqwestMcpHttpTransport {
    /// Builds a transport that returns redirect responses without following them.
    pub fn new() -> Result<Self> {
        let client = match reqwest::Client::builder().redirect(Policy::none()).build() {
            Ok(client) => client,
            Err(source) => return Err(Error::transport(&source)),
        };
        Ok(Self { client })
    }
}

#[async_trait]
impl McpHttpTransport for ReqwestMcpHttpTransport {
    async fn post(
        &self,
        url: &str,
        headers: &BTreeMap<String, String>,
        body: &Value,
        max_response_bytes: usize,
        timeout: Duration,
    ) -> Result<McpHttpResponse> {
        let builder = apply_headers(
            self.client.request(Method::POST, url).timeout(timeout),
            headers,
        )
        .json(body);
        let response = send(builder).await?;
        decode_response(response, max_response_bytes, has_session_header(headers)).await
    }

    async fn delete(
        &self,
        url: &str,
        headers: &BTreeMap<String, String>,
        max_response_bytes: usize,
        timeout: Duration,
    ) -> Result<McpHttpResponse> {
        let builder = apply_headers(
            self.client.request(Method::DELETE, url).timeout(timeout),
            headers,
        );
        let response = send(builder).await?;
        decode_delete_response(response, max_response_bytes, has_session_header(headers)).await
    }
}

fn apply_headers(
    mut builder: reqwest::RequestBuilder,
    headers: &BTreeMap<String, String>,
) -> reqwest::RequestBuilder {
    for (name, value) in headers {
        builder = builder.header(name, value);
    }
    builder
}

async fn send(builder: reqwest::RequestBuilder) -> Result<Response> {
    match builder.send().await {
        Ok(response) => Ok(response),
        Err(source) => Err(Error::transport(&source)),
    }
}

async fn decode_response(
    response: Response,
    limit_bytes: usize,
    session_bound: bool,
) -> Result<McpHttpResponse> {
    let status = response.status().as_u16();
    let headers = response_headers(&response);
    if is_authorization_status(status) || is_expired_session_status(status, session_bound) {
        return Ok(McpHttpResponse {
            status,
            headers,
            payload: McpHttpPayload::None,
        });
    }
    if matches(&headers, EVENT_STREAM) {
        return Ok(McpHttpResponse {
            status,
            headers,
            payload: McpHttpPayload::EventStream(Box::new(ReqwestEventStream::new(
                response.bytes_stream(),
                limit_bytes,
            ))),
        });
    }
    let body = read_capped(response, limit_bytes).await?;
    let payload = if body.is_empty() {
        McpHttpPayload::None
    } else {
        McpHttpPayload::Json(parse_body(&body))
    };
    Ok(McpHttpResponse {
        status,
        headers,
        payload,
    })
}

async fn decode_delete_response(
    response: Response,
    limit_bytes: usize,
    session_bound: bool,
) -> Result<McpHttpResponse> {
    let status = response.status().as_u16();
    if is_tolerated_delete_status(status) {
        return Ok(McpHttpResponse {
            status,
            headers: response_headers(&response),
            payload: McpHttpPayload::None,
        });
    }
    decode_response(response, limit_bytes, session_bound).await
}

fn response_headers(response: &Response) -> BTreeMap<String, Vec<String>> {
    let mut headers = BTreeMap::new();
    for name in response.headers().keys() {
        let values = response
            .headers()
            .get_all(name)
            .iter()
            .map(|value| String::from_utf8_lossy(value.as_bytes()).into_owned())
            .collect();
        headers.insert(name.as_str().to_ascii_lowercase(), values);
    }
    headers
}

async fn read_capped(response: Response, limit_bytes: usize) -> Result<Vec<u8>> {
    let mut bytes = Vec::new();
    let mut stream = response.bytes_stream();
    while let Some(chunk) = stream.next().await {
        let chunk = match chunk {
            Ok(chunk) => chunk,
            Err(source) => return Err(Error::transport(&source)),
        };
        if bytes.len().saturating_add(chunk.len()) > limit_bytes {
            return Err(Error::ResponseTooLarge { limit_bytes });
        }
        bytes.extend_from_slice(&chunk);
    }
    Ok(bytes)
}

fn parse_body(bytes: &[u8]) -> Value {
    match serde_json::from_slice(bytes) {
        Ok(value) => value,
        Err(_) => Value::String(String::from_utf8_lossy(bytes).into_owned()),
    }
}
