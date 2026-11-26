//! Decisions redirect handling through the production HTTP transport.

use std::{collections::BTreeMap, sync::Arc};

use ai_interface::{ErrorKind, JudgmentModel, JudgmentQuestion, JudgmentRequest};
use ai_models_core::ProviderConfig;
use ai_models_openai::{GPT_6_LUNA, OpenAiJudgmentModel, PROVIDER_ID};
use json_http::ReqwestJsonHttpClient;
use tokio::{
    io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader},
    net::{TcpListener, TcpStream},
    sync::oneshot,
};

const DECISIONS_REQUEST: &str = "POST /v1/decisions HTTP/1.1";
const REDIRECT_BODY: &str =
    r#"{"error":{"code":"insufficient_quota","message":"gateway redirect detail"}}"#;
const TARGET_BODY: &str =
    r#"{"model":"gpt-6-luna","answers":[{"name":"q0","type":"predicate","probability":0.9}]}"#;

#[tokio::test]
async fn temporary_redirect_does_not_resend_the_decisions_post() {
    assert_redirect_stops_after_post("307 Temporary Redirect", REDIRECT_BODY.len()).await;
}

#[tokio::test]
async fn found_redirect_does_not_send_a_followup_get() {
    assert_redirect_stops_after_post("302 Found", REDIRECT_BODY.len()).await;
}

#[tokio::test]
async fn truncated_redirect_preserves_the_decisions_redirect_error() {
    assert_redirect_stops_after_post("307 Temporary Redirect", REDIRECT_BODY.len() + 1).await;
}

async fn assert_redirect_stops_after_post(status: &'static str, content_length: usize) {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let base_url = format!("http://{}", listener.local_addr().unwrap());
    let (finish, finished) = oneshot::channel();
    let server = tokio::spawn(serve(listener, status, content_length, finished));
    let model = OpenAiJudgmentModel::from_config(
        Arc::new(ReqwestJsonHttpClient::new()),
        GPT_6_LUNA,
        ProviderConfig::new(PROVIDER_ID, "fixture-api-key").with_base_url(base_url),
    );

    let result = model
        .judge(&JudgmentRequest {
            input: "A ticket is delayed.".into(),
            questions: BTreeMap::from([(
                "urgent".to_owned(),
                JudgmentQuestion::condition("Is it urgent?", None),
            )]),
        })
        .await;
    finish.send(()).unwrap();
    let requests = server.await.unwrap();

    assert_eq!(requests, [DECISIONS_REQUEST], "{status}");
    let error = result.unwrap_err();
    assert_eq!(error.core().kind, ErrorKind::InvalidOutput);
    assert_eq!(error.core().message, "unexpected HTTP redirect");
    assert_eq!(
        error.core().provider_request_id.as_deref(),
        Some("req-redirect")
    );
}

async fn serve(
    listener: TcpListener,
    redirect_status: &str,
    redirect_content_length: usize,
    mut finished: oneshot::Receiver<()>,
) -> Vec<String> {
    let mut requests = Vec::new();
    loop {
        let (mut socket, _) = tokio::select! {
            biased;
            connection = listener.accept() => connection.unwrap(),
            _ = &mut finished => return requests,
        };
        let request = read_request_line(&mut socket).await;
        let (status, headers, body, length) = if request == DECISIONS_REQUEST {
            (
                redirect_status,
                "Location: /target\r\nX-Request-Id: req-redirect\r\n",
                REDIRECT_BODY,
                redirect_content_length,
            )
        } else {
            ("200 OK", "", TARGET_BODY, TARGET_BODY.len())
        };
        requests.push(request);
        let response = format!(
            "HTTP/1.1 {status}\r\nConnection: close\r\nContent-Type: application/json\r\n{headers}Content-Length: {length}\r\n\r\n{body}"
        );
        socket.write_all(response.as_bytes()).await.unwrap();
        socket.shutdown().await.unwrap();
    }
}

async fn read_request_line(socket: &mut TcpStream) -> String {
    let mut reader = BufReader::new(socket);
    let mut request = String::new();
    assert_ne!(reader.read_line(&mut request).await.unwrap(), 0);
    let mut length = 0;
    let mut header = String::new();
    loop {
        header.clear();
        assert_ne!(reader.read_line(&mut header).await.unwrap(), 0);
        if header == "\r\n" {
            break;
        }
        if let Some((name, value)) = header.split_once(':')
            && name.eq_ignore_ascii_case("content-length")
        {
            length = value.trim().parse().unwrap();
        }
    }
    reader.read_exact(&mut vec![0; length]).await.unwrap();
    request.trim_end().to_owned()
}
