//! Live SSE streamable-HTTP integration coverage through reqwest.

mod support;

use std::{
    convert::Infallible,
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
    },
};

use ai_mcp::McpClient;
use axum::{
    Json, Router,
    body::{Body, Bytes},
    extract::State,
    http::{HeaderMap, HeaderValue, StatusCode, header::CONTENT_TYPE},
    response::{IntoResponse, Response},
    routing::post,
};
use json_http::StaticHeaderAuth;
use serde_json::{Value, json};
use tokio::sync::{Notify, mpsc};
use tokio_stream::wrappers::ReceiverStream;

use support::{RecordedRequest, client, header, spawn};

#[derive(Default)]
struct SseServerState {
    posts: Mutex<Vec<RecordedRequest>>,
    side_reply_seen: AtomicBool,
    side_reply: Notify,
}

#[tokio::test]
async fn runs_live_sse_flow_and_replies_before_stream_completion() {
    let state = Arc::new(SseServerState::default());
    let router = Router::new()
        .route("/mcp", post(post_mcp))
        .with_state(state.clone());
    let server = spawn(router).await;
    let client = client(&server.endpoint, Arc::new(StaticHeaderAuth::default()));

    client.ensure_initialized().await.unwrap();
    let tools = client.list_tools().await.unwrap();
    let outcome = client.call_tool("stream", json!({})).await.unwrap();

    assert_eq!(tools[0].name, "stream");
    assert_eq!(outcome.structured_content, Some(json!({"streamed": true})));
    assert!(state.side_reply_seen.load(Ordering::SeqCst));
    assert!(client.tools_list_changed());
    client.list_tools().await.unwrap();
    assert!(!client.tools_list_changed());

    let posts = state.posts.lock().unwrap();
    let side_reply = posts
        .iter()
        .find(|request| request.body.get("method").is_none() && request.body["id"] == "server-1")
        .unwrap();
    assert_eq!(side_reply.body["result"], json!({}));
    assert_eq!(header(side_reply, "mcp-session-id"), "sse-session");
    assert_eq!(header(side_reply, "mcp-protocol-version"), "2025-06-18");
}

async fn post_mcp(
    State(state): State<Arc<SseServerState>>,
    headers: HeaderMap,
    Json(body): Json<Value>,
) -> Response {
    state.posts.lock().unwrap().push(RecordedRequest {
        headers,
        body: body.clone(),
    });
    let method = body.get("method").and_then(Value::as_str);
    if method.is_none() {
        state.side_reply_seen.store(true, Ordering::SeqCst);
        state.side_reply.notify_one();
        return StatusCode::ACCEPTED.into_response();
    }
    let id = body["id"].clone();
    match method {
        Some("initialize") => initialized(id),
        Some("notifications/initialized") => StatusCode::ACCEPTED.into_response(),
        Some("tools/list") => sse_response(vec![json!({
            "jsonrpc": "2.0",
            "id": id,
            "result": {
                "tools": [{"name": "stream", "description": "Stream", "inputSchema": {}}]
            }
        })]),
        Some("tools/call") => live_call_response(state, id),
        _ => StatusCode::BAD_REQUEST.into_response(),
    }
}

fn initialized(id: Value) -> Response {
    let mut response = sse_response(vec![json!({
        "jsonrpc": "2.0",
        "id": id,
        "result": {
            "protocolVersion": "2025-06-18",
            "capabilities": {"tools": {"listChanged": true}},
            "serverInfo": {"name": "sse-server", "version": "1.0"}
        }
    })]);
    response
        .headers_mut()
        .insert("mcp-session-id", HeaderValue::from_static("sse-session"));
    response
}

fn live_call_response(state: Arc<SseServerState>, id: Value) -> Response {
    let (sender, receiver) = mpsc::channel::<Result<Bytes, Infallible>>(3);
    tokio::spawn(async move {
        sender
            .send(Ok(event(json!({
                "jsonrpc": "2.0",
                "id": "server-1",
                "method": "ping"
            }))))
            .await
            .unwrap();
        state.side_reply.notified().await;
        sender
            .send(Ok(event(json!({
                "jsonrpc": "2.0",
                "method": "notifications/tools/list_changed"
            }))))
            .await
            .unwrap();
        sender
            .send(Ok(event(json!({
                "jsonrpc": "2.0",
                "id": id,
                "result": {
                    "content": [{"type": "text", "text": "done"}],
                    "structuredContent": {"streamed": true}
                }
            }))))
            .await
            .unwrap();
    });
    Response::builder()
        .header(CONTENT_TYPE, "text/event-stream")
        .body(Body::from_stream(ReceiverStream::new(receiver)))
        .unwrap()
}

fn sse_response(messages: Vec<Value>) -> Response {
    let body = messages
        .into_iter()
        .map(event)
        .fold(Vec::new(), |mut bytes, event| {
            bytes.extend_from_slice(&event);
            bytes
        });
    Response::builder()
        .header(CONTENT_TYPE, "text/event-stream")
        .body(Body::from(body))
        .unwrap()
}

fn event(message: Value) -> Bytes {
    Bytes::from(format!("data: {message}\n\n"))
}
