use std::time::Duration;

use json_http::{Error, JsonHttpClient, ReqwestJsonHttpClient};
use serde_json::json;

use super::support::{BodyFraming, ResponseSpec, ResponseStep, spawn_server};

#[tokio::test]
async fn non_success_status_retains_a_bounded_json_body() {
    let body = br#"{"error":{"message":"slow down"}}"#.to_vec();
    let server = spawn_server(ResponseSpec {
        status: "429 Too Many Requests",
        content_type: Some("application/json"),
        header_delay: Duration::ZERO,
        framing: BodyFraming::Fixed {
            declared_length: body.len(),
        },
        steps: vec![ResponseStep::Bytes(body)],
    })
    .await;
    let client = ReqwestJsonHttpClient::new();

    let result = client.get(&server.url).send_sse().await;

    assert!(matches!(
        result,
        Err(Error::HttpStatus { status: 429, body })
            if body == json!({ "error": { "message": "slow down" } })
    ));
    server.request().await;
}

#[tokio::test]
async fn non_success_text_body_is_capped_at_sixty_four_kibibytes() {
    let body = vec![b'x'; 70 * 1024];
    let server = spawn_server(ResponseSpec {
        status: "500 Internal Server Error",
        content_type: Some("text/plain"),
        header_delay: Duration::ZERO,
        framing: BodyFraming::Fixed {
            declared_length: body.len(),
        },
        steps: vec![ResponseStep::Bytes(body)],
    })
    .await;
    let client = ReqwestJsonHttpClient::new();

    let result = client.get(&server.url).send_sse().await;

    assert!(matches!(
        result,
        Err(Error::HttpStatus { status: 500, body })
            if body.as_str().is_some_and(|body| body.len() == 64 * 1024)
    ));
    server.request().await;
}

#[tokio::test]
async fn successful_non_sse_content_type_is_rejected() {
    let body = b"{}".to_vec();
    let server = spawn_server(ResponseSpec {
        status: "200 OK",
        content_type: Some("application/json"),
        header_delay: Duration::ZERO,
        framing: BodyFraming::Fixed {
            declared_length: body.len(),
        },
        steps: vec![ResponseStep::Bytes(body)],
    })
    .await;
    let client = ReqwestJsonHttpClient::new();

    let result = client.get(&server.url).send_sse().await;

    assert!(matches!(
        result,
        Err(Error::InvalidSseContentType { content_type })
            if content_type.as_deref() == Some("application/json")
    ));
    server.request().await;
}

#[tokio::test]
async fn malformed_utf8_reports_decoder_progress() {
    let server = spawn_server(ResponseSpec::sse(vec![ResponseStep::Bytes(
        b"data: \xff\n\n".to_vec(),
    )]))
    .await;
    let client = ReqwestJsonHttpClient::new();
    let mut stream = client.get(&server.url).send_sse().await.unwrap();

    assert!(matches!(
        stream.next().await,
        Err(Error::SseDecode {
            events_received: 0,
            ..
        })
    ));
    server.request().await;
}

#[tokio::test]
async fn broken_body_records_progress_before_transport_failure() {
    let bytes = b"data: first\n\n".to_vec();
    let server = spawn_server(ResponseSpec {
        status: "200 OK",
        content_type: Some("text/event-stream"),
        header_delay: Duration::ZERO,
        framing: BodyFraming::Fixed {
            declared_length: bytes.len() + 100,
        },
        steps: vec![ResponseStep::Bytes(bytes)],
    })
    .await;
    let client = ReqwestJsonHttpClient::new();
    let mut stream = client.get(&server.url).send_sse().await.unwrap();

    assert!(stream.next().await.unwrap().is_some());
    assert!(matches!(
        stream.next().await,
        Err(Error::SseTransport {
            events_received: 1,
            ..
        })
    ));
    server.request().await;
}
