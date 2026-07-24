//! Revocation and unconditional local-disconnect tests.

use std::sync::{Arc, Mutex};

use serde_json::Value;
use unimock::{MockFn, Unimock, matching};

use crate::{
    Error, McpOAuthConfig, McpOAuthDiscoveryMock, McpOAuthManager, OAuthCredentialStoreMock,
    OAuthHttpResponse, OAuthHttpTransportMock, OAuthStoreOperation,
};

use super::support::{key, manager, server_metadata, tokens};

#[tokio::test]
async fn revokes_a_refresh_token_then_deletes_local_tokens() {
    let captured = Arc::new(Mutex::new(None::<Vec<(String, String)>>));
    let store = Unimock::new((
        OAuthCredentialStoreMock::load_tokens
            .next_call(matching!(_))
            .returns(Ok(Some(tokens(
                "access-secret",
                Some("refresh-secret"),
                None,
            )))),
        OAuthCredentialStoreMock::delete_tokens
            .next_call(matching!(_))
            .returns(Ok(())),
    ));
    let transport = Unimock::new(
        OAuthHttpTransportMock::post_form
            .next_call(matching!(_, _, _, _, _))
            .answers_arc({
                let captured = captured.clone();
                Arc::new(move |_, _, _, _, _, fields: &[(String, String)]| {
                    *captured.lock().unwrap() = Some(fields.to_vec());
                    Ok(OAuthHttpResponse {
                        status: 200,
                        headers: Default::default(),
                        body: Value::Null,
                    })
                })
            }),
    );

    disconnect_manager(store, transport, server_metadata())
        .disconnect(&key("account"))
        .await
        .unwrap();

    let fields = captured.lock().unwrap().clone().unwrap();
    assert_eq!(field(&fields, "token"), "refresh-secret");
    assert_eq!(field(&fields, "token_type_hint"), "refresh_token");
    assert_eq!(field(&fields, "client_id"), "client-id");
}

#[tokio::test]
async fn revocation_failure_still_deletes_local_tokens() {
    let store = Unimock::new((
        OAuthCredentialStoreMock::load_tokens
            .next_call(matching!(_))
            .returns(Ok(Some(tokens("access", Some("refresh"), None)))),
        OAuthCredentialStoreMock::delete_tokens
            .next_call(matching!(_))
            .returns(Ok(())),
    ));
    let transport = Unimock::new(
        OAuthHttpTransportMock::post_form
            .next_call(matching!(_, _, _, _, _))
            .returns(Err(Error::Transport)),
    );

    let error = disconnect_manager(store, transport, server_metadata())
        .disconnect(&key("account"))
        .await
        .unwrap_err();

    assert!(matches!(error, Error::RevocationFailed));
}

#[tokio::test]
async fn local_deletion_failure_remains_distinct() {
    let store = Unimock::new((
        OAuthCredentialStoreMock::load_tokens
            .next_call(matching!(_))
            .returns(Ok(Some(tokens("access", Some("refresh"), None)))),
        OAuthCredentialStoreMock::delete_tokens
            .next_call(matching!(_))
            .returns(Err(Error::Store {
                operation: OAuthStoreOperation::DeleteTokens,
            })),
    ));
    let transport = Unimock::new(
        OAuthHttpTransportMock::post_form
            .next_call(matching!(_, _, _, _, _))
            .returns(Ok(OAuthHttpResponse {
                status: 200,
                headers: Default::default(),
                body: Value::Null,
            })),
    );

    let error = disconnect_manager(store, transport, server_metadata())
        .disconnect(&key("account"))
        .await
        .unwrap_err();

    assert!(matches!(
        error,
        Error::LocalTokenDeletionFailed {
            revocation_failed: false
        }
    ));
}

#[tokio::test]
async fn missing_revocation_endpoint_skips_network_and_deletes_locally() {
    let store = Unimock::new((
        OAuthCredentialStoreMock::load_tokens
            .next_call(matching!(_))
            .returns(Ok(Some(tokens("access", None, None)))),
        OAuthCredentialStoreMock::delete_tokens
            .next_call(matching!(_))
            .returns(Ok(())),
    ));
    let mut server = server_metadata();
    server.revocation_endpoint = None;

    disconnect_manager(store, Unimock::new(()), server)
        .disconnect(&key("account"))
        .await
        .unwrap();
}

fn disconnect_manager(
    store: Unimock,
    transport: Unimock,
    server: crate::AuthorizationServerMetadata,
) -> crate::DefaultMcpOAuthManager {
    manager(
        Unimock::new(
            McpOAuthDiscoveryMock::authorization_server
                .next_call(matching!(_))
                .returns(Ok(server)),
        ),
        Unimock::new(()),
        store,
        Unimock::new(()),
        transport,
        Unimock::new(()),
        Unimock::new(()),
        McpOAuthConfig::default(),
    )
}

fn field<'a>(fields: &'a [(String, String)], name: &str) -> &'a str {
    fields
        .iter()
        .find(|(candidate, _)| candidate == name)
        .map(|(_, value)| value.as_str())
        .unwrap()
}
