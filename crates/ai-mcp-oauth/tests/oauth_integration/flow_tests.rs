//! Credential-free end-to-end authorization and authenticated MCP lifecycle.

use std::time::Duration;

use ai_mcp::{Error as McpError, McpClient};
use ai_mcp_oauth::McpOAuthManager;
use secrecy::ExposeSecret;
use serde_json::json;

use super::support::{
    FakeOAuthMcpServer, authenticated_client, credential_key, fresh_expiry, harness, tokens,
    unauthenticated_client,
};

#[tokio::test]
async fn credential_free_oauth_mcp_smoke_retries_initialize_once() {
    let server = FakeOAuthMcpServer::spawn().await;
    let harness = harness(&server);
    let error = unauthenticated_client(&server)
        .ensure_initialized()
        .await
        .unwrap_err();
    let challenge = match error {
        McpError::AuthorizationRequired { challenge } => challenge,
        other => panic!("expected authorization challenge, got {other:?}"),
    };

    let connection = harness
        .manager
        .authorize(&challenge, &harness.context)
        .await
        .unwrap();
    let client = authenticated_client(&server, &harness, connection.key.clone());
    let handshake = client.ensure_initialized().await.unwrap();

    assert_eq!(handshake.server_info.name, "oauth-test");
    assert_eq!(harness.store.registration_count().await, 1);
    let stored = harness.store.tokens(&connection.key).await.unwrap();
    assert_eq!(stored.access_token.expose_secret(), "access-1");
    assert_eq!(harness.user_agent.call_count(), 1);

    let records = server.records().await;
    assert_eq!(records.registrations.len(), 1);
    assert_eq!(records.authorization_queries.len(), 1);
    assert_eq!(records.token_forms.len(), 1);
    let query = &records.authorization_queries[0];
    assert_eq!(query["response_type"], "code");
    assert_eq!(query["resource"], server.mcp_url);
    assert_eq!(query["scope"], "read");
    assert_eq!(query["code_challenge_method"], "S256");
    let token = &records.token_forms[0];
    assert_eq!(token["grant_type"], "authorization_code");
    assert_eq!(token["redirect_uri"], "http://127.0.0.1/callback");
    assert_eq!(token["resource"], server.mcp_url);
    assert_eq!(token["code"], "authorization-code");
    assert_eq!(token["code_verifier"].len(), 43);

    let initializes = records
        .mcp_requests
        .iter()
        .filter(|request| request.body.as_ref().and_then(method) == Some("initialize"))
        .collect::<Vec<_>>();
    assert_eq!(initializes.len(), 2);
    assert_eq!(initializes[0].authorization, None);
    assert_eq!(
        initializes[1].authorization.as_deref(),
        Some("Bearer access-1")
    );
}

#[tokio::test]
async fn reuses_one_stored_token_for_posts_sse_responses_and_delete() {
    let server = FakeOAuthMcpServer::spawn().await;
    let harness = harness(&server);
    let key = credential_key(&server);
    harness
        .store
        .insert_tokens(
            key.clone(),
            tokens(
                "access-1",
                Some("refresh-1"),
                Some(fresh_expiry()),
                &["read"],
            ),
        )
        .await;
    let client = authenticated_client(&server, &harness, key);

    client.ensure_initialized().await.unwrap();
    client.list_tools().await.unwrap();
    let outcome = client.call_tool("sse", json!({})).await.unwrap();
    client.close().await.unwrap();

    assert_eq!(outcome.content.len(), 1);
    assert_eq!(harness.user_agent.call_count(), 0);
    let records = server.records().await;
    assert!(records.mcp_requests.len() >= 6);
    assert!(
        records
            .mcp_requests
            .iter()
            .all(|request| { request.authorization.as_deref() == Some("Bearer access-1") })
    );
    assert!(records.mcp_requests.iter().any(|request| {
        request.http_method == "POST"
            && request
                .body
                .as_ref()
                .is_some_and(|body| body.get("id") == Some(&json!("server-ping")))
    }));
    assert!(
        records
            .mcp_requests
            .iter()
            .any(|request| request.http_method == "DELETE")
    );
}

#[tokio::test]
async fn concurrent_mcp_calls_share_one_real_refresh() {
    let server = FakeOAuthMcpServer::spawn().await;
    server
        .configure(|behavior| behavior.refresh_delay = Duration::from_millis(25))
        .await;
    let harness = harness(&server);
    let key = credential_key(&server);
    harness
        .store
        .insert_tokens(
            key.clone(),
            tokens(
                "access-1",
                Some("refresh-1"),
                Some(fresh_expiry()),
                &["read"],
            ),
        )
        .await;
    let client = authenticated_client(&server, &harness, key.clone());
    client.ensure_initialized().await.unwrap();
    harness
        .store
        .insert_tokens(
            key.clone(),
            tokens("access-1", Some("refresh-1"), Some(1), &["read"]),
        )
        .await;

    let (left, right) = tokio::join!(
        client.call_tool("echo", json!({"side": "left"})),
        client.call_tool("echo", json!({"side": "right"}))
    );

    assert!(left.is_ok());
    assert!(right.is_ok());
    let records = server.records().await;
    assert_eq!(
        records
            .token_forms
            .iter()
            .filter(|form| form["grant_type"] == "refresh_token")
            .count(),
        1
    );
    let tool_requests = records
        .mcp_requests
        .iter()
        .filter(|request| request.body.as_ref().and_then(method) == Some("tools/call"))
        .collect::<Vec<_>>();
    assert_eq!(tool_requests.len(), 2);
    assert!(
        tool_requests
            .iter()
            .all(|request| { request.authorization.as_deref() == Some("Bearer access-2") })
    );
    assert_eq!(
        harness
            .store
            .tokens(&key)
            .await
            .unwrap()
            .access_token
            .expose_secret(),
        "access-2"
    );
    assert_eq!(harness.user_agent.call_count(), 0);
}

fn method(body: &serde_json::Value) -> Option<&str> {
    body.get("method").and_then(serde_json::Value::as_str)
}
