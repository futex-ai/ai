//! Authorization challenge integration coverage through the reqwest transport.

mod support;

use std::{
    collections::BTreeMap,
    sync::{Arc, Mutex},
    time::Duration,
};

use ai_mcp::{
    Error, McpAuthorizationFailure, McpClient, McpHttpPayload, McpHttpTransport,
    ReqwestMcpHttpTransport,
};
use axum::{
    Json, Router,
    extract::State,
    http::{HeaderMap, HeaderValue, StatusCode, header::WWW_AUTHENTICATE},
    response::{IntoResponse, Response},
    routing::post,
};
use json_http::StaticHeaderAuth;
use serde_json::{Value, json};

use support::{RecordedRequest, client, header, spawn};

const AUTH_RESPONSE_LIMIT: usize = 64;
const OVERSIZED_BODY_BYTES: usize = 128;

#[derive(Default)]
struct AuthServerState {
    requests: Mutex<Vec<RecordedRequest>>,
}

#[tokio::test]
async fn preserves_repeated_401_challenges_despite_oversized_body() {
    let state = Arc::new(AuthServerState::default());
    let server = spawn(
        Router::new()
            .route("/mcp", post(unauthorized))
            .with_state(state.clone()),
    )
    .await;
    let client = client(
        &server.endpoint,
        Arc::new(StaticHeaderAuth::default()),
        Some(AUTH_RESPONSE_LIMIT),
    );

    let error = client.ensure_initialized().await.unwrap_err();

    let challenge = match error {
        Error::AuthorizationRequired { challenge } => challenge,
        other => panic!("expected authorization challenge, got {other:?}"),
    };
    assert_eq!(challenge.failure, McpAuthorizationFailure::InvalidToken);
    assert_eq!(
        challenge.resource_metadata_url.as_deref(),
        Some("https://resource.example/.well-known/oauth-protected-resource")
    );
    assert_eq!(challenge.scopes, ["read", "write"]);
    assert_eq!(challenge.raw_www_authenticate.len(), 2);
    assert!(challenge.raw_www_authenticate[1].starts_with("Basic"));
    let requests = state.requests.lock().unwrap();
    assert_eq!(requests[0].body["method"], "initialize");
    assert_eq!(header(&requests[0], "content-type"), "application/json");
}

#[tokio::test]
async fn maps_403_scope_challenge_despite_oversized_body() {
    let state = Arc::new(AuthServerState::default());
    let server = spawn(
        Router::new()
            .route("/mcp", post(forbidden))
            .with_state(state.clone()),
    )
    .await;
    let client = client(
        &server.endpoint,
        Arc::new(StaticHeaderAuth::bearer_token("insufficient-token")),
        Some(AUTH_RESPONSE_LIMIT),
    );

    let error = client.ensure_initialized().await.unwrap_err();

    let challenge = match error {
        Error::Forbidden { challenge } => challenge,
        other => panic!("expected forbidden challenge, got {other:?}"),
    };
    assert_eq!(
        challenge.failure,
        McpAuthorizationFailure::InsufficientScope
    );
    assert_eq!(challenge.scopes, ["admin", "read"]);
    assert_eq!(challenge.raw_www_authenticate.len(), 2);
    let requests = state.requests.lock().unwrap();
    assert_eq!(requests[0].body["method"], "initialize");
    assert_eq!(
        header(&requests[0], "authorization"),
        "Bearer insufficient-token"
    );
}

#[tokio::test]
async fn transport_returns_headers_without_reading_oversized_401_body() {
    let state = Arc::new(AuthServerState::default());
    let server = spawn(
        Router::new()
            .route("/mcp", post(unauthorized))
            .with_state(state),
    )
    .await;

    let response = ReqwestMcpHttpTransport::new()
        .unwrap()
        .post(
            &server.endpoint,
            &BTreeMap::new(),
            &json!({"jsonrpc": "2.0", "id": 1, "method": "initialize"}),
            AUTH_RESPONSE_LIMIT,
            Duration::from_secs(2),
        )
        .await
        .unwrap();

    assert_eq!(response.status, 401);
    assert_eq!(response.headers["www-authenticate"].len(), 2);
    assert!(matches!(response.payload, McpHttpPayload::None));
}

async fn unauthorized(
    State(state): State<Arc<AuthServerState>>,
    headers: HeaderMap,
    Json(body): Json<Value>,
) -> Response {
    record(&state, headers, body);
    let mut response = (StatusCode::UNAUTHORIZED, "x".repeat(OVERSIZED_BODY_BYTES)).into_response();
    response.headers_mut().append(
        WWW_AUTHENTICATE,
        HeaderValue::from_static(
            "Bearer error=\"invalid_token\", scope=\"read write\", resource_metadata=\"https://resource.example/.well-known/oauth-protected-resource\"",
        ),
    );
    response.headers_mut().append(
        WWW_AUTHENTICATE,
        HeaderValue::from_static("Basic realm=\"fallback\""),
    );
    response
}

async fn forbidden(
    State(state): State<Arc<AuthServerState>>,
    headers: HeaderMap,
    Json(body): Json<Value>,
) -> Response {
    record(&state, headers, body);
    let mut response = (StatusCode::FORBIDDEN, "x".repeat(OVERSIZED_BODY_BYTES)).into_response();
    response.headers_mut().append(
        WWW_AUTHENTICATE,
        HeaderValue::from_static("Bearer error=\"insufficient_scope\", scope=\"admin read\""),
    );
    response.headers_mut().append(
        WWW_AUTHENTICATE,
        HeaderValue::from_static("Basic realm=\"fallback\""),
    );
    response
}

fn record(state: &AuthServerState, headers: HeaderMap, body: Value) {
    state
        .requests
        .lock()
        .unwrap()
        .push(RecordedRequest { headers, body });
}
