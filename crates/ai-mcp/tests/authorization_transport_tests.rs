//! Authorization challenge integration coverage through the reqwest transport.

mod support;

use std::sync::{Arc, Mutex};

use ai_mcp::{Error, McpAuthorizationFailure, McpClient};
use axum::{
    Json, Router,
    extract::State,
    http::{HeaderMap, HeaderValue, StatusCode, header::WWW_AUTHENTICATE},
    response::{IntoResponse, Response},
    routing::post,
};
use json_http::StaticHeaderAuth;
use serde_json::Value;

use support::{RecordedRequest, client, header, spawn};

#[derive(Default)]
struct AuthServerState {
    requests: Mutex<Vec<RecordedRequest>>,
}

#[tokio::test]
async fn preserves_repeated_401_challenges_and_typed_discovery_hints() {
    let state = Arc::new(AuthServerState::default());
    let server = spawn(
        Router::new()
            .route("/mcp", post(unauthorized))
            .with_state(state.clone()),
    )
    .await;
    let client = client(&server.endpoint, Arc::new(StaticHeaderAuth::default()));

    let error = client.ensure_initialized().await.unwrap_err();

    let Error::AuthorizationRequired { challenge } = error else {
        panic!("expected authorization challenge");
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
async fn maps_403_scope_challenge_without_losing_raw_headers() {
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
    );

    let error = client.ensure_initialized().await.unwrap_err();

    let Error::Forbidden { challenge } = error else {
        panic!("expected forbidden challenge");
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

async fn unauthorized(
    State(state): State<Arc<AuthServerState>>,
    headers: HeaderMap,
    Json(body): Json<Value>,
) -> Response {
    record(&state, headers, body);
    let mut response = StatusCode::UNAUTHORIZED.into_response();
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
    let mut response = StatusCode::FORBIDDEN.into_response();
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
