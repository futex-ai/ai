//! Status-aware handling of SSE-typed HTTP error bodies.

#[expect(
    dead_code,
    reason = "shared helpers are compiled per integration test; this suite uses only client/spawn"
)]
mod support;

use std::{collections::BTreeMap, sync::Arc, time::Duration};

use ai_mcp::{Error, McpClient, McpHttpPayload, McpHttpTransport, ReqwestMcpHttpTransport};
use axum::{
    Router,
    http::{StatusCode, header::CONTENT_TYPE},
    response::{IntoResponse, Response},
    routing::{delete, post},
};
use json_http::StaticHeaderAuth;
use serde_json::{Value, json};

use support::{client, spawn};

const SMALL_DIAGNOSTIC: &str = "data: {\"error\":\"busy\"}\n\n";
const ERROR_BODY_LIMIT: usize = 64;

#[tokio::test]
async fn client_preserves_small_sse_429_error_body() {
    let server = spawn(Router::new().route("/mcp", post(too_many_requests))).await;
    let client = client(
        &server.endpoint,
        Arc::new(StaticHeaderAuth::default()),
        None,
    );

    let error = client.ensure_initialized().await.unwrap_err();

    assert!(matches!(
        error,
        Error::HttpStatus {
            status: 429,
            body: Value::String(body),
        } if body == SMALL_DIAGNOSTIC
    ));
}

#[tokio::test]
async fn non_session_sse_404_preserves_diagnostic() {
    let server = spawn(Router::new().route("/mcp", post(not_found))).await;
    let client = client(
        &server.endpoint,
        Arc::new(StaticHeaderAuth::default()),
        None,
    );

    let error = client.ensure_initialized().await.unwrap_err();

    assert!(matches!(
        error,
        Error::HttpStatus {
            status: 404,
            body: Value::String(body),
        } if body == SMALL_DIAGNOSTIC
    ));
}

#[tokio::test]
async fn oversized_sse_post_error_is_bounded() {
    let server = spawn(Router::new().route("/mcp", post(oversized_server_error))).await;
    let transport = ReqwestMcpHttpTransport::new().unwrap();

    let error = match transport
        .post(
            &server.endpoint,
            &BTreeMap::new(),
            &json!({"jsonrpc": "2.0", "id": 1, "method": "initialize"}),
            ERROR_BODY_LIMIT,
            Duration::from_secs(2),
        )
        .await
    {
        Ok(_) => panic!("expected an oversized POST error"),
        Err(error) => error,
    };

    assert!(matches!(
        error,
        Error::ResponseTooLarge {
            limit_bytes: ERROR_BODY_LIMIT
        }
    ));
}

#[tokio::test]
async fn delete_buffers_small_sse_error_body() {
    let server = spawn(Router::new().route("/mcp", delete(server_error))).await;
    let response = ReqwestMcpHttpTransport::new()
        .unwrap()
        .delete(
            &server.endpoint,
            &BTreeMap::new(),
            ERROR_BODY_LIMIT,
            Duration::from_secs(2),
        )
        .await
        .unwrap();

    assert_eq!(response.status, 500);
    assert!(matches!(
        response.payload,
        McpHttpPayload::Json(Value::String(body)) if body == SMALL_DIAGNOSTIC
    ));
}

#[tokio::test]
async fn oversized_sse_delete_error_is_bounded() {
    let server = spawn(Router::new().route("/mcp", delete(oversized_server_error))).await;
    let transport = ReqwestMcpHttpTransport::new().unwrap();

    let error = match transport
        .delete(
            &server.endpoint,
            &BTreeMap::new(),
            ERROR_BODY_LIMIT,
            Duration::from_secs(2),
        )
        .await
    {
        Ok(_) => panic!("expected an oversized DELETE error"),
        Err(error) => error,
    };

    assert!(matches!(
        error,
        Error::ResponseTooLarge {
            limit_bytes: ERROR_BODY_LIMIT
        }
    ));
}

async fn too_many_requests() -> Response {
    sse_error(StatusCode::TOO_MANY_REQUESTS, SMALL_DIAGNOSTIC)
}

async fn not_found() -> Response {
    sse_error(StatusCode::NOT_FOUND, SMALL_DIAGNOSTIC)
}

async fn server_error() -> Response {
    sse_error(StatusCode::INTERNAL_SERVER_ERROR, SMALL_DIAGNOSTIC)
}

async fn oversized_server_error() -> Response {
    sse_error(
        StatusCode::INTERNAL_SERVER_ERROR,
        &format!("data: {}\n\n", "x".repeat(ERROR_BODY_LIMIT * 2)),
    )
}

fn sse_error(status: StatusCode, body: &str) -> Response {
    (
        status,
        [(CONTENT_TYPE, "text/event-stream")],
        body.to_owned(),
    )
        .into_response()
}
