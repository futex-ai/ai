//! Invalid-grant, post-refresh denial, and revocation integration coverage.

use ai_mcp::{Error as McpError, McpClient};
use ai_mcp_oauth::McpOAuthManager;
use secrecy::ExposeSecret;

use super::support::{FakeOAuthMcpServer, authenticated_client, credential_key, harness, tokens};

#[tokio::test]
async fn invalid_grant_clears_tokens_and_surfaces_one_challenge() {
    let server = FakeOAuthMcpServer::spawn().await;
    server
        .configure(|behavior| behavior.refresh_invalid_grant = true)
        .await;
    let harness = harness(&server);
    let key = credential_key(&server);
    harness
        .store
        .insert_tokens(
            key.clone(),
            tokens("expired", Some("invalid-refresh"), Some(1), &["read"]),
        )
        .await;
    let client = authenticated_client(&server, &harness, key.clone());

    let error = client.ensure_initialized().await.unwrap_err();

    assert!(matches!(error, McpError::AuthorizationRequired { .. }));
    assert!(harness.store.tokens(&key).await.is_none());
    assert_eq!(harness.store.token_deletion_count(), 1);
    assert_eq!(harness.user_agent.call_count(), 0);
    let records = server.records().await;
    assert_eq!(records.token_forms.len(), 1);
    assert_eq!(records.mcp_requests.len(), 1);
    assert_eq!(records.mcp_requests[0].authorization, None);
}

#[tokio::test]
async fn post_refresh_unauthorized_is_not_retried_or_interactive() {
    let server = FakeOAuthMcpServer::spawn().await;
    server
        .configure(|behavior| behavior.reject_authorized = true)
        .await;
    let harness = harness(&server);
    let key = credential_key(&server);
    harness
        .store
        .insert_tokens(
            key.clone(),
            tokens("expired", Some("refresh-1"), Some(1), &["read"]),
        )
        .await;
    let client = authenticated_client(&server, &harness, key.clone());

    let error = client.ensure_initialized().await.unwrap_err();

    assert!(matches!(error, McpError::AuthorizationRequired { .. }));
    assert_eq!(harness.user_agent.call_count(), 0);
    let stored = harness.store.tokens(&key).await.unwrap();
    assert_eq!(stored.access_token.expose_secret(), "access-2");
    let records = server.records().await;
    assert_eq!(records.token_forms.len(), 1);
    assert_eq!(records.mcp_requests.len(), 1);
    assert_eq!(
        records.mcp_requests[0].authorization.as_deref(),
        Some("Bearer access-2")
    );
}

#[tokio::test]
async fn disconnect_revokes_refresh_token_and_removes_local_tokens() {
    let server = FakeOAuthMcpServer::spawn().await;
    let harness = harness(&server);
    let key = credential_key(&server);
    harness
        .store
        .insert_tokens(
            key.clone(),
            tokens("access-1", Some("refresh-1"), None, &["read"]),
        )
        .await;

    harness.manager.disconnect(&key).await.unwrap();

    assert!(harness.store.tokens(&key).await.is_none());
    let records = server.records().await;
    assert_eq!(records.revocation_forms.len(), 1);
    assert_eq!(records.revocation_forms[0]["token"], "refresh-1");
    assert_eq!(
        records.revocation_forms[0]["token_type_hint"],
        "refresh_token"
    );
}
