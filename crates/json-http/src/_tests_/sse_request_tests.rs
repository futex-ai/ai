//! Tests for SSE request assembly and mockable stream boundaries.

use std::{sync::Arc, time::Duration};

use async_trait::async_trait;
use serde_json::{Value, json};
use unimock::{MockFn, Unimock, matching};

use crate::{
    DynJsonHttpSseStream, Error, JsonHttpClient, JsonHttpRequest, JsonHttpResponse,
    JsonHttpSseEvent, JsonHttpSseStream, JsonHttpSseStreamMock, JsonHttpTransport,
    JsonHttpTransportMock, StaticHeaderAuth, TransportBackedJsonHttpClient,
};

struct EmptyStream;

#[async_trait]
impl JsonHttpSseStream for EmptyStream {
    async fn next(&mut self) -> crate::Result<Option<JsonHttpSseEvent>> {
        Ok(None)
    }
}

struct UnsupportedTransport;

#[async_trait]
impl JsonHttpTransport for UnsupportedTransport {
    async fn execute(&self, _request: &JsonHttpRequest) -> crate::Result<JsonHttpResponse<Value>> {
        Err(Error::transport("unexpected buffered request"))
    }

    async fn execute_bytes(
        &self,
        _request: &JsonHttpRequest,
    ) -> crate::Result<JsonHttpResponse<Vec<u8>>> {
        Err(Error::transport("unexpected byte request"))
    }
}

#[tokio::test]
async fn request_defaults_match_the_documented_timeout_contract() {
    let transport = Arc::new(Unimock::new(
        JsonHttpTransportMock::execute
            .next_call(matching!(_))
            .answers(&|_, request: &JsonHttpRequest| {
                assert_eq!(request.timeout, Duration::from_secs(600));
                assert_eq!(request.idle_timeout, None);
                Ok(JsonHttpResponse {
                    status: 200,
                    body: json!({}),
                })
            }),
    ));
    let client = TransportBackedJsonHttpClient::new(transport);

    client
        .get("https://example.com")
        .send_value()
        .await
        .unwrap();
}

#[tokio::test]
async fn sse_builder_applies_auth_body_and_timeout_controls() {
    let transport = Arc::new(Unimock::new(
        JsonHttpTransportMock::execute_sse
            .next_call(matching!(_))
            .answers(&|_, request: &JsonHttpRequest| {
                assert_eq!(request.timeout, Duration::from_secs(90));
                assert_eq!(request.idle_timeout, Some(Duration::from_secs(12)));
                assert_eq!(
                    request.headers.get("Authorization").map(String::as_str),
                    Some("Bearer token")
                );
                assert_eq!(
                    request.body.as_ref().and_then(|body| body.get("stream")),
                    Some(&json!(true))
                );
                Ok(Box::new(EmptyStream) as DynJsonHttpSseStream)
            }),
    ));
    let client = TransportBackedJsonHttpClient::new(transport);

    let mut stream = client
        .post("https://example.com/v1/completions")
        .auth(Arc::new(StaticHeaderAuth::bearer_token("token")))
        .timeout(Duration::from_secs(90))
        .idle_timeout(Duration::from_secs(12))
        .json(json!({ "stream": true }))
        .unwrap()
        .send_sse()
        .await
        .unwrap();

    assert_eq!(stream.next().await.unwrap(), None);
}

#[tokio::test]
async fn transport_default_reports_sse_unsupported() {
    let client = TransportBackedJsonHttpClient::new(Arc::new(UnsupportedTransport));

    let result = client
        .post("https://example.com/v1/completions")
        .send_sse()
        .await;

    assert!(matches!(result, Err(Error::SseUnsupported)));
}

#[tokio::test]
async fn sse_stream_trait_has_a_unimock_boundary() {
    let mut stream = Unimock::new(JsonHttpSseStreamMock::next.next_call(matching!()).answers(
        &|_| {
            Ok(Some(JsonHttpSseEvent {
                event: Some("done".to_owned()),
                data: "complete".to_owned(),
            }))
        },
    ));

    assert_eq!(
        stream.next().await.unwrap(),
        Some(JsonHttpSseEvent {
            event: Some("done".to_owned()),
            data: "complete".to_owned(),
        })
    );
}
