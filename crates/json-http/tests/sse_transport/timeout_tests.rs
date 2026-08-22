use std::time::Duration;

use json_http::{Error, JsonHttpClient, ReqwestJsonHttpClient};

use super::support::{ResponseSpec, ResponseStep, spawn_server};

#[tokio::test]
async fn idle_timeout_applies_while_opening_the_stream() {
    let mut spec = ResponseSpec::sse(vec![]);
    spec.header_delay = Duration::from_millis(200);
    let server = spawn_server(spec).await;
    let client = ReqwestJsonHttpClient::new();

    let result = client
        .get(&server.url)
        .timeout(Duration::from_secs(1))
        .idle_timeout(Duration::from_millis(40))
        .send_sse()
        .await;

    assert!(matches!(
        result,
        Err(Error::IdleTimeout {
            idle,
            events_received: 0
        }) if idle == Duration::from_millis(40)
    ));
    server.request().await;
}

#[tokio::test]
async fn idle_timeout_records_events_already_received() {
    let server = spawn_server(ResponseSpec::sse(vec![
        ResponseStep::Bytes(b"data: first\n\n".to_vec()),
        ResponseStep::Delay(Duration::from_millis(200)),
    ]))
    .await;
    let client = ReqwestJsonHttpClient::new();
    let mut stream = client
        .get(&server.url)
        .timeout(Duration::from_secs(1))
        .idle_timeout(Duration::from_millis(40))
        .send_sse()
        .await
        .unwrap();

    assert!(stream.next().await.unwrap().is_some());
    assert!(matches!(
        stream.next().await,
        Err(Error::IdleTimeout {
            idle,
            events_received: 1
        }) if idle == Duration::from_millis(40)
    ));
    server.request().await;
}

#[tokio::test]
async fn overall_deadline_is_not_reset_by_an_event() {
    let server = spawn_server(ResponseSpec::sse(vec![
        ResponseStep::Bytes(b"data: first\n\n".to_vec()),
        ResponseStep::Delay(Duration::from_millis(250)),
    ]))
    .await;
    let client = ReqwestJsonHttpClient::new();
    let mut stream = client
        .get(&server.url)
        .timeout(Duration::from_millis(80))
        .idle_timeout(Duration::from_secs(1))
        .send_sse()
        .await
        .unwrap();

    assert!(stream.next().await.unwrap().is_some());
    assert!(matches!(
        stream.next().await,
        Err(Error::DeadlineExceeded {
            timeout,
            events_received: 1
        }) if timeout == Duration::from_millis(80)
    ));
    server.request().await;
}

#[tokio::test]
async fn incomplete_event_bytes_do_not_reset_the_idle_timeout() {
    let server = spawn_server(ResponseSpec::sse(vec![
        ResponseStep::Bytes(b"data: par".to_vec()),
        ResponseStep::Delay(Duration::from_millis(25)),
        ResponseStep::Bytes(b"ti".to_vec()),
        ResponseStep::Delay(Duration::from_millis(40)),
        ResponseStep::Bytes(b"al\n\n".to_vec()),
    ]))
    .await;
    let client = ReqwestJsonHttpClient::new();
    let mut stream = client
        .get(&server.url)
        .timeout(Duration::from_secs(1))
        .idle_timeout(Duration::from_millis(50))
        .send_sse()
        .await
        .unwrap();

    assert!(matches!(
        stream.next().await,
        Err(Error::IdleTimeout {
            idle,
            events_received: 0
        }) if idle == Duration::from_millis(50)
    ));
    server.request().await;
}
