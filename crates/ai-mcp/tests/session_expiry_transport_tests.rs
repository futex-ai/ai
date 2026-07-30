//! Status-authoritative session-expiry coverage through the reqwest transport.

mod support;

use std::{
    collections::BTreeMap,
    sync::{
        Arc, Mutex,
        atomic::{AtomicUsize, Ordering},
    },
    time::Duration,
};

use ai_mcp::{Error, McpClient, McpHttpPayload, McpHttpTransport, ReqwestMcpHttpTransport};
use axum::{
    Json, Router,
    extract::State,
    http::{HeaderMap, HeaderValue, StatusCode, header::CONTENT_TYPE},
    response::{IntoResponse, Response},
    routing::{delete, post},
};
use json_http::StaticHeaderAuth;
use serde_json::{Value, json};

use support::{RecordedRequest, client, header, spawn};

const CLIENT_RESPONSE_LIMIT: usize = 512;
const OVERSIZED_BODY_BYTES: usize = 1024;

#[derive(Default)]
struct RecoveryState {
    initializations: AtomicUsize,
    requests: Mutex<Vec<RecordedRequest>>,
}

#[tokio::test]
async fn oversized_session_404_allows_host_retry_with_a_new_session() {
    let state = Arc::new(RecoveryState::default());
    let server = spawn(
        Router::new()
            .route("/mcp", post(recovery_post))
            .with_state(state.clone()),
    )
    .await;
    let client = client(
        &server.endpoint,
        Arc::new(StaticHeaderAuth::default()),
        Some(CLIENT_RESPONSE_LIMIT),
    );

    client.ensure_initialized().await.unwrap();
    let error = client.list_tools().await.unwrap_err();

    assert!(matches!(error, Error::SessionExpired));
    assert!(client.tools_list_changed());
    assert!(client.list_tools().await.unwrap().is_empty());
    let requests = state.requests.lock().unwrap();
    assert_eq!(
        requests
            .iter()
            .filter(|request| request.body["method"] == "initialize")
            .count(),
        2
    );
    assert_eq!(header(&requests[2], "mcp-session-id"), "session-1");
    assert!(!requests[3].headers.contains_key("mcp-session-id"));
    assert_eq!(header(&requests[5], "mcp-session-id"), "session-2");
}

#[tokio::test]
async fn session_bound_post_404_ignores_an_oversized_body() {
    let server = spawn(Router::new().route("/mcp", post(oversized_not_found))).await;
    let response = ReqwestMcpHttpTransport::new()
        .unwrap()
        .post(
            &server.endpoint,
            &session_headers("Mcp-Session-Id", "expired-session"),
            &json!({"jsonrpc": "2.0", "id": 1, "method": "tools/list"}),
            1,
            Duration::from_secs(2),
        )
        .await
        .unwrap();

    assert_eq!(response.status, 404);
    assert_eq!(
        response.headers.get("x-expired-session"),
        Some(&vec!["true".to_owned()])
    );
    assert!(matches!(response.payload, McpHttpPayload::None));
}

#[tokio::test]
async fn lowercase_session_header_makes_delete_404_bodyless() {
    let server = spawn(Router::new().route("/mcp", delete(oversized_not_found))).await;
    let response = ReqwestMcpHttpTransport::new()
        .unwrap()
        .delete(
            &server.endpoint,
            &session_headers("mcp-session-id", "expired-session"),
            1,
            Duration::from_secs(2),
        )
        .await
        .unwrap();

    assert_eq!(response.status, 404);
    assert_eq!(
        response.headers.get("x-expired-session"),
        Some(&vec!["true".to_owned()])
    );
    assert!(matches!(response.payload, McpHttpPayload::None));
}

#[tokio::test]
async fn empty_session_header_still_makes_post_404_bodyless() {
    let server = spawn(Router::new().route("/mcp", post(oversized_not_found))).await;
    let response = ReqwestMcpHttpTransport::new()
        .unwrap()
        .post(
            &server.endpoint,
            &session_headers("Mcp-Session-Id", ""),
            &json!({"jsonrpc": "2.0", "id": 1, "method": "tools/list"}),
            1,
            Duration::from_secs(2),
        )
        .await
        .unwrap();

    assert_eq!(response.status, 404);
    assert!(matches!(response.payload, McpHttpPayload::None));
}

#[tokio::test]
async fn session_bound_sse_404_is_bodyless() {
    let server = spawn(Router::new().route("/mcp", post(sse_not_found))).await;
    let response = ReqwestMcpHttpTransport::new()
        .unwrap()
        .post(
            &server.endpoint,
            &session_headers("Mcp-Session-Id", "expired-session"),
            &json!({"jsonrpc": "2.0", "id": 1, "method": "tools/list"}),
            1,
            Duration::from_secs(2),
        )
        .await
        .unwrap();

    assert_eq!(response.status, 404);
    assert!(matches!(response.payload, McpHttpPayload::None));
}

#[tokio::test]
async fn non_session_post_and_delete_404s_keep_bounded_bodies() {
    let server = spawn(Router::new().route(
        "/mcp",
        post(oversized_not_found).delete(oversized_not_found),
    ))
    .await;
    let transport = ReqwestMcpHttpTransport::new().unwrap();

    let post_error = match transport
        .post(
            &server.endpoint,
            &BTreeMap::new(),
            &json!({"jsonrpc": "2.0", "id": 1, "method": "initialize"}),
            1,
            Duration::from_secs(2),
        )
        .await
    {
        Ok(_) => panic!("expected a bounded POST body failure"),
        Err(error) => error,
    };
    let delete_error = match transport
        .delete(
            &server.endpoint,
            &BTreeMap::new(),
            1,
            Duration::from_secs(2),
        )
        .await
    {
        Ok(_) => panic!("expected a bounded DELETE body failure"),
        Err(error) => error,
    };

    assert!(matches!(
        post_error,
        Error::ResponseTooLarge { limit_bytes: 1 }
    ));
    assert!(matches!(
        delete_error,
        Error::ResponseTooLarge { limit_bytes: 1 }
    ));
}

async fn recovery_post(
    State(state): State<Arc<RecoveryState>>,
    headers: HeaderMap,
    Json(body): Json<Value>,
) -> Response {
    state.requests.lock().unwrap().push(RecordedRequest {
        headers: headers.clone(),
        body: body.clone(),
    });
    let id = body["id"].clone();
    match body.get("method").and_then(Value::as_str) {
        Some("initialize") => {
            let generation = state.initializations.fetch_add(1, Ordering::SeqCst) + 1;
            initialized(id, &format!("session-{generation}"))
        }
        Some("notifications/initialized") => StatusCode::ACCEPTED.into_response(),
        Some("tools/list")
            if headers
                .get("mcp-session-id")
                .is_some_and(|value| value == "session-1") =>
        {
            oversized_not_found().await
        }
        Some("tools/list") => Json(json!({
            "jsonrpc": "2.0",
            "id": id,
            "result": {"tools": []}
        }))
        .into_response(),
        _ => StatusCode::BAD_REQUEST.into_response(),
    }
}

fn initialized(id: Value, session_id: &str) -> Response {
    let mut response = Json(json!({
        "jsonrpc": "2.0",
        "id": id,
        "result": {
            "protocolVersion": "2025-06-18",
            "capabilities": {},
            "serverInfo": {"name": "expiry-server", "version": "1"}
        }
    }))
    .into_response();
    response
        .headers_mut()
        .insert("mcp-session-id", HeaderValue::from_str(session_id).unwrap());
    response
}

async fn oversized_not_found() -> Response {
    (
        StatusCode::NOT_FOUND,
        [("x-expired-session", "true")],
        "x".repeat(OVERSIZED_BODY_BYTES),
    )
        .into_response()
}

async fn sse_not_found() -> Response {
    let mut response = (
        StatusCode::NOT_FOUND,
        "data: {\"jsonrpc\":\"2.0\",\"id\":1,\"result\":{}}\n\n",
    )
        .into_response();
    response
        .headers_mut()
        .insert(CONTENT_TYPE, HeaderValue::from_static("text/event-stream"));
    response
}

fn session_headers(name: &str, value: &str) -> BTreeMap<String, String> {
    BTreeMap::from([(name.to_owned(), value.to_owned())])
}
