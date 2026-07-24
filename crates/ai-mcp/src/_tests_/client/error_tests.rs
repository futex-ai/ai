//! Typed client error tests.

use std::{collections::BTreeMap, sync::Arc};

use async_trait::async_trait;
use json_http::{JsonHttpAuth, StaticHeaderAuth};
use serde_json::json;

use crate::{Error, McpAuthorizationFailure, McpClient, McpServerConfig, StreamableHttpMcpClient};

use super::{
    lifecycle_tests::initialized_response,
    support::{ScriptedTransport, empty_response, json_response},
};

#[tokio::test]
async fn maps_repeated_authorization_challenges() {
    let transport = ScriptedTransport::new(vec![json_response(
        401,
        json!({"error":"unauthorized"}),
        BTreeMap::from([(
            "www-authenticate".to_owned(),
            vec![
                "Bearer error=\"invalid_token\", scope=\"read write\"".to_owned(),
                "Bearer resource_metadata=\"https://example.com/meta\"".to_owned(),
            ],
        )]),
    )]);
    let client = client(transport);

    let error = client.ensure_initialized().await.unwrap_err();

    assert!(matches!(
        error,
        Error::AuthorizationRequired { challenge }
            if challenge.failure == McpAuthorizationFailure::InvalidToken
                && challenge.scopes == ["read", "write"]
                && challenge.raw_www_authenticate.len() == 2
    ));
}

#[tokio::test]
async fn rejects_unsupported_negotiated_versions() {
    let transport = ScriptedTransport::new(vec![json_response(
        200,
        json!({
            "jsonrpc":"2.0","id":1,
            "result":{
                "protocolVersion":"2024-11-05",
                "capabilities":{},
                "serverInfo":{"name":"old","version":"1"}
            }
        }),
        BTreeMap::new(),
    )]);
    let error = client(transport).ensure_initialized().await.unwrap_err();

    assert!(
        matches!(error, Error::UnsupportedProtocolVersion { server, .. } if server == "2024-11-05")
    );
}

#[tokio::test]
async fn treats_session_bound_404_as_expired() {
    let transport = ScriptedTransport::new(vec![
        json_response(
            200,
            initialized_response_payload(),
            BTreeMap::from([("mcp-session-id".to_owned(), vec!["s".to_owned()])]),
        ),
        empty_response(202),
        json_response(404, json!({"error":"gone"}), BTreeMap::new()),
    ]);
    let client = client(transport);

    client.ensure_initialized().await.unwrap();
    let error = client.list_tools().await.unwrap_err();

    assert!(matches!(error, Error::SessionExpired));
}

#[tokio::test]
async fn distinguishes_forbidden_insufficient_scope() {
    let transport = ScriptedTransport::new(vec![json_response(
        403,
        json!({"error":"forbidden"}),
        BTreeMap::from([(
            "www-authenticate".to_owned(),
            vec!["Bearer error=\"insufficient_scope\", scope=\"admin\"".to_owned()],
        )]),
    )]);
    let error = client(transport).ensure_initialized().await.unwrap_err();

    assert!(matches!(
        error,
        Error::Forbidden { challenge }
            if challenge.failure == McpAuthorizationFailure::InsufficientScope
                && challenge.scopes == ["admin"]
    ));
}

#[tokio::test]
async fn surfaces_auth_hook_failures_without_dispatching() {
    let transport = ScriptedTransport::new(Vec::new());
    let client = StreamableHttpMcpClient::new(
        transport.clone(),
        Arc::new(FailingAuth),
        McpServerConfig::new("demo", "https://example.com/mcp"),
    )
    .unwrap();

    let error = client.ensure_initialized().await.unwrap_err();

    assert!(matches!(error, Error::Auth { .. }));
    assert!(transport.posts().is_empty());
}

#[tokio::test]
async fn preserves_other_http_status_and_body() {
    let transport = ScriptedTransport::new(vec![json_response(
        429,
        json!({"retry":"later"}),
        BTreeMap::new(),
    )]);

    let error = client(transport).ensure_initialized().await.unwrap_err();

    assert!(matches!(
        error,
        Error::HttpStatus { status: 429, body } if body == json!({"retry":"later"})
    ));
}

fn client(transport: Arc<ScriptedTransport>) -> StreamableHttpMcpClient {
    StreamableHttpMcpClient::new(
        transport,
        Arc::new(StaticHeaderAuth::default()),
        McpServerConfig::new("demo", "https://example.com/mcp"),
    )
    .unwrap()
}

fn initialized_response_payload() -> serde_json::Value {
    match initialized_response().payload {
        crate::McpHttpPayload::Json(value) => value,
        _ => unreachable!(),
    }
}

struct FailingAuth;

#[async_trait]
impl JsonHttpAuth for FailingAuth {
    async fn apply_headers(
        &self,
        _headers: &mut BTreeMap<String, String>,
    ) -> json_http::Result<()> {
        Err(json_http::Error::auth("auth unavailable"))
    }
}
