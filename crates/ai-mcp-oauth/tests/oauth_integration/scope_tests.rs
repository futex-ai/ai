//! Incremental-scope consent, denial suppression, and ordinary 403 behavior.

use ai_mcp::{Error as McpError, McpAuthorizationChallenge, McpClient};
use ai_mcp_oauth::{Error, McpOAuthManager};
use serde_json::json;

use super::support::{
    FakeOAuthMcpServer, authenticated_client, configured_registration, credential_key,
    fresh_expiry, harness, tokens,
};

#[tokio::test]
async fn insufficient_scope_requests_minimum_scopes_and_accepts_granted_subset() {
    let server = FakeOAuthMcpServer::spawn().await;
    let mut harness = harness(&server);
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
    server
        .configure(|behavior| {
            behavior.insufficient_scope = true;
            behavior.granted_scope = Some("write".to_owned());
        })
        .await;
    let challenge =
        insufficient_scope_challenge(client.call_tool("echo", json!({})).await.unwrap_err());
    harness.context.configured_registration = Some(configured_registration());

    let connection = harness
        .manager
        .authorize(&challenge, &harness.context)
        .await
        .unwrap();
    let outcome = client.call_tool("echo", json!({})).await.unwrap();

    assert_eq!(connection.scopes.as_slice(), &["write"]);
    assert_eq!(outcome.content.len(), 1);
    let records = server.records().await;
    assert_eq!(records.authorization_queries.len(), 1);
    assert_eq!(records.authorization_queries[0]["scope"], "read write");
    assert_eq!(records.authorization_queries[0]["resource"], server.mcp_url);
    assert_eq!(harness.user_agent.call_count(), 1);
}

#[tokio::test]
async fn denied_incremental_scope_is_not_prompted_twice_for_one_attempt() {
    let server = FakeOAuthMcpServer::spawn().await;
    let mut harness = harness(&server);
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
    server
        .configure(|behavior| {
            behavior.insufficient_scope = true;
            behavior.deny_authorization = true;
        })
        .await;
    let challenge =
        insufficient_scope_challenge(client.call_tool("echo", json!({})).await.unwrap_err());
    harness.context.configured_registration = Some(configured_registration());

    let first = harness
        .manager
        .authorize(&challenge, &harness.context)
        .await;
    let second = harness
        .manager
        .authorize(&challenge, &harness.context)
        .await;

    assert!(matches!(first, Err(Error::UserDenied)));
    assert!(matches!(second, Err(Error::UserDenied)));
    assert_eq!(harness.user_agent.call_count(), 1);
    let records = server.records().await;
    assert_eq!(records.authorization_queries.len(), 1);
    assert!(records.token_forms.is_empty());
}

#[tokio::test]
async fn ordinary_forbidden_response_never_opens_the_user_agent() {
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
    server.configure(|behavior| behavior.forbidden = true).await;
    let challenge = match client.call_tool("echo", json!({})).await.unwrap_err() {
        McpError::Forbidden { challenge } => challenge,
        other => panic!("expected forbidden challenge, got {other:?}"),
    };

    let error = harness
        .manager
        .authorize(&challenge, &harness.context)
        .await
        .unwrap_err();

    assert!(matches!(error, Error::AuthorizationForbidden));
    assert_eq!(harness.user_agent.call_count(), 0);
    assert!(server.records().await.authorization_queries.is_empty());
}

fn insufficient_scope_challenge(error: McpError) -> McpAuthorizationChallenge {
    match error {
        McpError::Forbidden { challenge } => challenge,
        other => panic!("expected insufficient-scope challenge, got {other:?}"),
    }
}
