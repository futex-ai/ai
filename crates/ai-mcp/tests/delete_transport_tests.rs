//! Status-authoritative session DELETE integration coverage.

use std::{
    collections::BTreeMap,
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    },
    time::Duration,
};

use ai_mcp::{
    McpClient, McpHttpPayload, McpHttpTransport, McpServerConfig, ReqwestMcpHttpTransport,
    StreamableHttpMcpClient,
};
use axum::{
    Json, Router,
    extract::State,
    http::{HeaderValue, StatusCode, header::WWW_AUTHENTICATE},
    response::{IntoResponse, Response},
    routing::{delete, post},
};
use json_http::StaticHeaderAuth;
use serde_json::{Value, json};
use tokio::{net::TcpListener, task::JoinHandle};

#[tokio::test]
async fn tolerated_delete_statuses_ignore_oversized_bodies() {
    for status in [StatusCode::OK, StatusCode::METHOD_NOT_ALLOWED] {
        let server = spawn(Router::new().route(
            "/mcp",
            delete(move || async move { (status, [("x-cleanup", "accepted")], "x".repeat(128)) }),
        ))
        .await;

        let response = ReqwestMcpHttpTransport::new()
            .unwrap()
            .delete(
                &server.endpoint,
                &BTreeMap::new(),
                1,
                Duration::from_secs(2),
            )
            .await
            .unwrap();

        assert_eq!(response.status, status.as_u16());
        assert_eq!(
            response.headers.get("x-cleanup"),
            Some(&vec!["accepted".to_owned()])
        );
        assert!(matches!(response.payload, McpHttpPayload::None));
    }
}

#[tokio::test]
async fn delete_authorization_status_ignores_an_oversized_body() {
    let server = spawn(Router::new().route(
        "/mcp",
        delete(|| async {
            let mut response = (StatusCode::UNAUTHORIZED, "x".repeat(128)).into_response();
            response.headers_mut().append(
                WWW_AUTHENTICATE,
                HeaderValue::from_static("Bearer error=\"invalid_token\""),
            );
            response
        }),
    ))
    .await;

    let response = ReqwestMcpHttpTransport::new()
        .unwrap()
        .delete(
            &server.endpoint,
            &BTreeMap::new(),
            1,
            Duration::from_secs(2),
        )
        .await
        .unwrap();

    assert_eq!(response.status, 401);
    assert_eq!(
        response.headers.get("www-authenticate"),
        Some(&vec!["Bearer error=\"invalid_token\"".to_owned()])
    );
    assert!(matches!(response.payload, McpHttpPayload::None));
}

#[tokio::test]
async fn close_clears_session_after_ignoring_an_oversized_body() {
    let state = Arc::new(CloseServerState::default());
    let server = spawn(
        Router::new()
            .route("/mcp", post(post_session).delete(delete_session))
            .with_state(state.clone()),
    )
    .await;
    let mut config = McpServerConfig::new("close", &server.endpoint);
    config.max_response_bytes = 512;
    let client = StreamableHttpMcpClient::new(
        Arc::new(ReqwestMcpHttpTransport::new().unwrap()),
        Arc::new(StaticHeaderAuth::default()),
        config,
    )
    .unwrap();
    client.ensure_initialized().await.unwrap();

    client.close().await.unwrap();
    client.close().await.unwrap();

    assert_eq!(state.deletes.load(Ordering::SeqCst), 1);
}

#[derive(Default)]
struct CloseServerState {
    deletes: AtomicUsize,
}

async fn post_session(Json(body): Json<Value>) -> Response {
    if body.get("method").and_then(Value::as_str) != Some("initialize") {
        return StatusCode::ACCEPTED.into_response();
    }
    let mut response = Json(json!({
        "jsonrpc": "2.0",
        "id": body["id"],
        "result": {
            "protocolVersion": "2025-06-18",
            "capabilities": {},
            "serverInfo": {"name": "close-server", "version": "1"}
        }
    }))
    .into_response();
    response
        .headers_mut()
        .insert("mcp-session-id", HeaderValue::from_static("close-session"));
    response
}

async fn delete_session(State(state): State<Arc<CloseServerState>>) -> Response {
    state.deletes.fetch_add(1, Ordering::SeqCst);
    (StatusCode::OK, "x".repeat(1024)).into_response()
}

struct TestServer {
    endpoint: String,
    task: JoinHandle<()>,
}

impl Drop for TestServer {
    fn drop(&mut self) {
        self.task.abort();
    }
}

async fn spawn(router: Router) -> TestServer {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let task = tokio::spawn(async move {
        axum::serve(listener, router).await.unwrap();
    });
    TestServer {
        endpoint: format!("http://{address}/mcp"),
        task,
    }
}
