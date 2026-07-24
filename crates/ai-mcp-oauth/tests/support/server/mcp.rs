//! Authenticated streamable-HTTP MCP routes.

use std::sync::Arc;

use axum::{
    Json,
    extract::State,
    http::{
        HeaderMap, HeaderValue, StatusCode,
        header::{CONTENT_TYPE, WWW_AUTHENTICATE},
    },
    response::{IntoResponse, Response},
};
use serde_json::{Value, json};

use super::{McpRequestRecord, ServerState};

pub(super) async fn post_request(
    State(state): State<Arc<ServerState>>,
    headers: HeaderMap,
    Json(body): Json<Value>,
) -> Response {
    let authorization = authorization(&headers);
    state
        .records
        .lock()
        .await
        .mcp_requests
        .push(McpRequestRecord {
            http_method: "POST",
            authorization: authorization.clone(),
            body: Some(body.clone()),
        });
    let behavior = state.behavior.lock().await.clone();
    if authorization.is_none() || behavior.reject_authorized {
        return challenge(&state.base_url, StatusCode::UNAUTHORIZED, false);
    }
    if method(&body) == Some("tools/call") {
        if behavior.forbidden {
            return challenge(&state.base_url, StatusCode::FORBIDDEN, false);
        }
        if behavior.insufficient_scope && authorization.as_deref() != Some("Bearer access-scope") {
            return challenge(&state.base_url, StatusCode::FORBIDDEN, true);
        }
    }
    success(body)
}

pub(super) async fn delete_request(
    State(state): State<Arc<ServerState>>,
    headers: HeaderMap,
) -> Response {
    state
        .records
        .lock()
        .await
        .mcp_requests
        .push(McpRequestRecord {
            http_method: "DELETE",
            authorization: authorization(&headers),
            body: None,
        });
    StatusCode::OK.into_response()
}

fn success(body: Value) -> Response {
    let Some(id) = body.get("id").cloned() else {
        return StatusCode::ACCEPTED.into_response();
    };
    match method(&body) {
        Some("initialize") => initialize(id),
        Some("tools/list") => Json(json!({
            "jsonrpc": "2.0",
            "id": id,
            "result": {"tools": [
                {"name": "echo", "inputSchema": {"type": "object"}},
                {"name": "sse", "inputSchema": {"type": "object"}}
            ]}
        }))
        .into_response(),
        Some("tools/call") if body["params"]["name"] == "sse" => sse_call(id),
        Some("tools/call") => Json(json!({
            "jsonrpc": "2.0",
            "id": id,
            "result": {"content": [{"type": "text", "text": "ok"}]}
        }))
        .into_response(),
        _ => StatusCode::ACCEPTED.into_response(),
    }
}

fn initialize(id: Value) -> Response {
    let mut response = Json(json!({
        "jsonrpc": "2.0",
        "id": id,
        "result": {
            "protocolVersion": "2025-06-18",
            "capabilities": {"tools": {}},
            "serverInfo": {"name": "oauth-test", "version": "1"}
        }
    }))
    .into_response();
    response
        .headers_mut()
        .insert("mcp-session-id", HeaderValue::from_static("oauth-session"));
    response
}

fn sse_call(id: Value) -> Response {
    let ping = json!({
        "jsonrpc": "2.0",
        "id": "server-ping",
        "method": "ping",
        "params": {}
    });
    let result = json!({
        "jsonrpc": "2.0",
        "id": id,
        "result": {"content": [{"type": "text", "text": "streamed"}]}
    });
    (
        [(CONTENT_TYPE, "text/event-stream")],
        format!("data: {ping}\n\ndata: {result}\n\n"),
    )
        .into_response()
}

fn challenge(base_url: &str, status: StatusCode, insufficient_scope: bool) -> Response {
    let error = if insufficient_scope {
        "error=\"insufficient_scope\", scope=\"write\", "
    } else {
        ""
    };
    let value = format!(
        "Bearer {error}resource_metadata=\"{base_url}/.well-known/oauth-protected-resource/mcp\""
    );
    let mut response = status.into_response();
    response
        .headers_mut()
        .insert(WWW_AUTHENTICATE, HeaderValue::from_str(&value).unwrap());
    response
}

fn authorization(headers: &HeaderMap) -> Option<String> {
    headers
        .get("authorization")
        .and_then(|value| value.to_str().ok())
        .map(str::to_owned)
}

fn method(body: &Value) -> Option<&str> {
    body.get("method").and_then(Value::as_str)
}
