//! JSON streamable-HTTP integration coverage through the reqwest transport.

mod support;

use std::{
    collections::BTreeMap,
    sync::{
        Arc, Mutex,
        atomic::{AtomicUsize, Ordering},
    },
    time::Duration,
};

use ai_mcp::{Error, McpClient, McpContentBlock, McpHttpTransport, ReqwestMcpHttpTransport};
use axum::{
    Json, Router,
    extract::State,
    http::{
        HeaderMap, HeaderValue, StatusCode,
        header::{CONTENT_TYPE, LOCATION},
    },
    response::{IntoResponse, Response},
    routing::{any, post},
};
use json_http::StaticHeaderAuth;
use serde_json::{Value, json};

use support::{RecordedRequest, client, header, spawn};

#[derive(Default)]
struct JsonServerState {
    posts: Mutex<Vec<RecordedRequest>>,
    deletes: Mutex<Vec<RecordedRequest>>,
}

#[tokio::test]
async fn rejects_json_body_with_unsupported_content_type() {
    let server = spawn(Router::new().route("/mcp", post(plain_text_json))).await;
    let client = client(&server.endpoint, Arc::new(StaticHeaderAuth::default()));

    let error = client.ensure_initialized().await.unwrap_err();

    assert!(matches!(
        error,
        Error::UnsupportedContentType {
            content_type: Some(content_type)
        } if content_type == "text/plain"
    ));
}

#[tokio::test]
async fn runs_json_session_from_initialize_through_close() {
    let state = Arc::new(JsonServerState::default());
    let router = Router::new()
        .route("/mcp", post(post_mcp).delete(delete_mcp))
        .with_state(state.clone());
    let server = spawn(router).await;
    let client = client(
        &server.endpoint,
        Arc::new(StaticHeaderAuth::bearer_token("integration-token")),
    );

    let handshake = client.ensure_initialized().await.unwrap();
    let tools = client.list_tools().await.unwrap();
    let outcome = client
        .call_tool("echo", json!({"message": "hello"}))
        .await
        .unwrap();
    client.close().await.unwrap();

    assert_eq!(handshake.server_info.name, "json-server");
    assert_eq!(
        tools
            .iter()
            .map(|tool| tool.name.as_str())
            .collect::<Vec<_>>(),
        ["first", "echo"]
    );
    assert!(matches!(
        outcome.content.as_slice(),
        [McpContentBlock::Text { text, .. }] if text == "hello"
    ));

    let posts = state.posts.lock().unwrap();
    assert_eq!(posts.len(), 5);
    assert_eq!(posts[0].body["method"], "initialize");
    assert_eq!(posts[1].body["method"], "notifications/initialized");
    assert_eq!(posts[2].body["method"], "tools/list");
    assert_eq!(posts[3].body["params"]["cursor"], "page-2");
    assert_eq!(posts[4].body["params"]["name"], "echo");
    for request in posts.iter() {
        assert_eq!(header(request, "authorization"), "Bearer integration-token");
    }
    assert!(posts[0].headers.get("mcp-session-id").is_none());
    for request in &posts[1..] {
        assert_eq!(header(request, "mcp-session-id"), "json-session");
        assert_eq!(header(request, "mcp-protocol-version"), "2025-06-18");
    }
    let deletes = state.deletes.lock().unwrap();
    assert_eq!(deletes.len(), 1);
    assert_eq!(header(&deletes[0], "mcp-session-id"), "json-session");
    assert_eq!(
        header(&deletes[0], "authorization"),
        "Bearer integration-token"
    );
}

#[tokio::test]
async fn post_redirects_surface_without_contacting_the_target() {
    for status in [StatusCode::FOUND, StatusCode::TEMPORARY_REDIRECT] {
        let (source, _target, target_hits) = redirect_server(status).await;
        let client = client(
            &source.endpoint,
            Arc::new(StaticHeaderAuth::bearer_token("integration-token")),
        );

        let error = client.ensure_initialized().await.unwrap_err();

        assert!(matches!(
            error,
            Error::HttpStatus {
                status: actual,
                ..
            } if actual == status.as_u16()
        ));
        assert_eq!(target_hits.load(Ordering::SeqCst), 0);
    }
}

#[tokio::test]
async fn delete_redirects_surface_without_contacting_the_target() {
    for status in [StatusCode::FOUND, StatusCode::TEMPORARY_REDIRECT] {
        let (source, _target, target_hits) = redirect_server(status).await;
        let response = ReqwestMcpHttpTransport::new()
            .unwrap()
            .delete(
                &source.endpoint,
                &BTreeMap::new(),
                1024,
                Duration::from_secs(2),
            )
            .await
            .unwrap();

        assert_eq!(response.status, status.as_u16());
        assert_eq!(target_hits.load(Ordering::SeqCst), 0);
    }
}

async fn plain_text_json(Json(body): Json<Value>) -> Response {
    if body.get("method").and_then(Value::as_str) != Some("initialize") {
        return StatusCode::ACCEPTED.into_response();
    }
    let mut response = Json(json!({
        "jsonrpc": "2.0",
        "id": body["id"],
        "result": {
            "protocolVersion": "2025-06-18",
            "capabilities": {},
            "serverInfo": {"name": "plain-json", "version": "1"}
        }
    }))
    .into_response();
    response
        .headers_mut()
        .insert(CONTENT_TYPE, HeaderValue::from_static("text/plain"));
    response
}

async fn post_mcp(
    State(state): State<Arc<JsonServerState>>,
    headers: HeaderMap,
    Json(body): Json<Value>,
) -> Response {
    state.posts.lock().unwrap().push(RecordedRequest {
        headers,
        body: body.clone(),
    });
    let id = body["id"].clone();
    match body.get("method").and_then(Value::as_str) {
        Some("initialize") => initialized(id),
        Some("notifications/initialized") => StatusCode::ACCEPTED.into_response(),
        Some("tools/list") => list_tools(id, &body),
        Some("tools/call") => Json(json!({
            "jsonrpc": "2.0",
            "id": id,
            "result": {
                "content": [{"type": "text", "text": body["params"]["arguments"]["message"]}]
            }
        }))
        .into_response(),
        _ => StatusCode::BAD_REQUEST.into_response(),
    }
}

async fn delete_mcp(State(state): State<Arc<JsonServerState>>, headers: HeaderMap) -> Response {
    state.deletes.lock().unwrap().push(RecordedRequest {
        headers,
        body: Value::Null,
    });
    (StatusCode::OK, "closed").into_response()
}

fn initialized(id: Value) -> Response {
    let mut response = Json(json!({
        "jsonrpc": "2.0",
        "id": id,
        "result": {
            "protocolVersion": "2025-06-18",
            "capabilities": {"tools": {"listChanged": true}},
            "serverInfo": {"name": "json-server", "version": "1.0"}
        }
    }))
    .into_response();
    response
        .headers_mut()
        .insert("mcp-session-id", HeaderValue::from_static("json-session"));
    response
}

fn list_tools(id: Value, request: &Value) -> Response {
    let cursor = request["params"].get("cursor").and_then(Value::as_str);
    let result = if cursor.is_none() {
        json!({
            "tools": [{"name": "first", "description": "First", "inputSchema": {}}],
            "nextCursor": "page-2"
        })
    } else {
        json!({
            "tools": [{"name": "echo", "description": "Echo", "inputSchema": {}}]
        })
    };
    Json(json!({"jsonrpc": "2.0", "id": id, "result": result})).into_response()
}

async fn redirect_server(
    status: StatusCode,
) -> (support::TestServer, support::TestServer, Arc<AtomicUsize>) {
    let target_hits = Arc::new(AtomicUsize::new(0));
    let target = spawn(Router::new().route(
        "/mcp",
        any({
            let target_hits = target_hits.clone();
            move || {
                let target_hits = target_hits.clone();
                async move {
                    target_hits.fetch_add(1, Ordering::SeqCst);
                    StatusCode::NO_CONTENT
                }
            }
        }),
    ))
    .await;
    let location = target.endpoint.clone();
    let source = spawn(Router::new().route(
        "/mcp",
        any(move || {
            let location = location.clone();
            async move { (status, [(LOCATION, location)]) }
        }),
    ))
    .await;

    (source, target, target_hits)
}
