use std::{sync::Arc, time::Duration};

use json_http::{JsonHttpClient, JsonHttpSseEvent, ReqwestJsonHttpClient, StaticHeaderAuth};
use serde_json::json;

use super::support::{BodyFraming, ResponseSpec, ResponseStep, spawn_server};

#[tokio::test]
async fn streams_split_events_and_applies_the_normal_request_builder() {
    let server = spawn_server(ResponseSpec::sse(vec![
        ResponseStep::Bytes(b"event: del".to_vec()),
        ResponseStep::Delay(Duration::from_millis(10)),
        ResponseStep::Bytes(b"ta\r\ndata: hel".to_vec()),
        ResponseStep::Bytes(b"lo\r\n\r\ndata: done".to_vec()),
    ]))
    .await;
    let client = ReqwestJsonHttpClient::new();
    let mut stream = client
        .post(&server.url)
        .auth(Arc::new(StaticHeaderAuth::bearer_token("token")))
        .idle_timeout(Duration::from_secs(1))
        .json(json!({ "stream": true }))
        .unwrap()
        .send_sse()
        .await
        .unwrap();

    assert_eq!(
        stream.next().await.unwrap(),
        Some(JsonHttpSseEvent {
            event: Some("delta".to_owned()),
            data: "hello".to_owned(),
        })
    );
    assert_eq!(
        stream.next().await.unwrap(),
        Some(JsonHttpSseEvent {
            event: None,
            data: "done".to_owned(),
        })
    );
    assert_eq!(stream.next().await.unwrap(), None);

    let request = server.request().await.to_ascii_lowercase();
    assert!(request.contains("authorization: bearer token"));
    assert!(request.contains("{\"stream\":true}"));
}

#[tokio::test]
async fn buffered_execution_ignores_the_sse_idle_timeout() {
    let body = br#"{"ok":true}"#.to_vec();
    let server = spawn_server(ResponseSpec {
        status: "200 OK",
        content_type: Some("application/json"),
        header_delay: Duration::ZERO,
        framing: BodyFraming::Fixed {
            declared_length: body.len(),
        },
        steps: vec![
            ResponseStep::Delay(Duration::from_millis(80)),
            ResponseStep::Bytes(body),
        ],
    })
    .await;
    let client = ReqwestJsonHttpClient::new();

    let response = client
        .get(&server.url)
        .timeout(Duration::from_secs(1))
        .idle_timeout(Duration::from_millis(20))
        .send_value()
        .await
        .unwrap();

    assert_eq!(response.body, json!({ "ok": true }));
    server.request().await;
}
