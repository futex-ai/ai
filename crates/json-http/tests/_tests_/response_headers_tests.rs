//! Response header retention across the three production response paths.

use std::time::Duration;

use json_http::{Error, JsonHttpClient, ReqwestJsonHttpClient};
use serde::Deserialize;
use serde_json::json;

use super::support::{BodyFraming, ResponseSpec, ResponseStep, spawn_server};

#[derive(Deserialize)]
struct ResponseBody {
    ok: bool,
}

#[tokio::test]
async fn buffered_and_byte_responses_keep_response_headers() {
    for binary in [false, true] {
        let body = br#"{"ok":true}"#.to_vec();
        let server = spawn_server(ResponseSpec {
            status: "200 OK",
            content_type: Some("application/json\r\nX-Request-Id: req-buffered\r\nX-Trace: first\r\nX-Trace: second"),
            header_delay: Duration::ZERO,
            framing: BodyFraming::Fixed { declared_length: body.len() },
            steps: vec![ResponseStep::Bytes(body.clone())],
        }).await;
        let client = ReqwestJsonHttpClient::new();
        let headers = if binary {
            let response = client.get(&server.url).send_bytes().await.unwrap();
            assert_eq!(response.body, body);
            response.headers
        } else {
            let response = client
                .get(&server.url)
                .send::<ResponseBody>()
                .await
                .unwrap();
            assert!(response.body.ok);
            response.headers
        };
        assert_eq!(
            headers
                .get("X-Request-Id")
                .and_then(|value| value.to_str().ok()),
            Some("req-buffered")
        );
        assert_eq!(
            headers
                .get_all("x-trace")
                .iter()
                .map(|value| value.to_str().unwrap())
                .collect::<Vec<_>>(),
            ["first", "second"]
        );
        server.request().await;
    }
}

#[tokio::test]
async fn successful_sse_opening_keeps_response_headers() {
    let mut response = ResponseSpec::sse(vec![ResponseStep::Bytes(b"data: ready\n\n".to_vec())]);
    response.content_type = Some("text/event-stream\r\nRequest-Id: req-stream");
    let server = spawn_server(response).await;
    let mut stream = ReqwestJsonHttpClient::new()
        .get(&server.url)
        .send_sse()
        .await
        .unwrap();
    assert_eq!(
        stream
            .headers()
            .get("request-id")
            .and_then(|value| value.to_str().ok()),
        Some("req-stream")
    );
    assert_eq!(stream.next().await.unwrap().unwrap().data, "ready");
    assert_eq!(
        stream
            .headers()
            .get("request-id")
            .and_then(|value| value.to_str().ok()),
        Some("req-stream")
    );
    assert_eq!(stream.next().await.unwrap(), None);
    server.request().await;
}

#[tokio::test]
async fn failed_sse_opening_keeps_status_headers_and_body() {
    let body = br#"{"error":"slow down"}"#.to_vec();
    let server = spawn_server(ResponseSpec {
        status: "429 Too Many Requests",
        content_type: Some("application/json\r\nRetry-After: 7\r\nRequest-Id: req-failed"),
        header_delay: Duration::ZERO,
        framing: BodyFraming::Fixed {
            declared_length: body.len(),
        },
        steps: vec![ResponseStep::Bytes(body)],
    })
    .await;
    let result = ReqwestJsonHttpClient::new()
        .get(&server.url)
        .send_sse()
        .await;
    let Err(Error::HttpStatus {
        status,
        headers,
        body,
    }) = result
    else {
        panic!("expected status error")
    };
    assert_eq!(status, 429);
    assert_eq!(body, json!({"error":"slow down"}));
    assert_eq!(
        headers
            .get("retry-after")
            .and_then(|value| value.to_str().ok()),
        Some("7")
    );
    assert_eq!(
        headers
            .get("request-id")
            .and_then(|value| value.to_str().ok()),
        Some("req-failed")
    );
    server.request().await;
}
