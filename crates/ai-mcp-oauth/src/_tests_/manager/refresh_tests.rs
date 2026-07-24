//! Refresh rotation, failure, skew, and single-flight tests.

use std::sync::{Arc, Mutex};

use secrecy::ExposeSecret;
use serde_json::json;
use unimock::{MockFn, Unimock, matching};

use crate::{
    Error, McpOAuthConfig, McpOAuthManager, OAuthCredentialStoreMock, OAuthHttpResponse,
    OAuthHttpTransportMock, OAuthStoreOperation, OAuthTokenSet,
};

use super::support::{clock, key, manager, refresh_manager, successful_refresh_transport, tokens};

#[tokio::test]
async fn explicit_refresh_rotates_tokens_and_granted_scopes() {
    let saved = Arc::new(Mutex::new(None::<OAuthTokenSet>));
    let submitted = Arc::new(Mutex::new(None::<Vec<(String, String)>>));
    let old = tokens("old-access", Some("old-refresh"), Some(100));
    let store = Unimock::new((
        OAuthCredentialStoreMock::load_tokens
            .next_call(matching!(_))
            .returns(Ok(Some(old.clone()))),
        OAuthCredentialStoreMock::load_tokens
            .next_call(matching!(_))
            .returns(Ok(Some(old))),
        OAuthCredentialStoreMock::save_tokens
            .next_call(matching!(_, _))
            .answers_arc({
                let saved = saved.clone();
                Arc::new(move |_, _, value: &OAuthTokenSet| {
                    *saved.lock().unwrap() = Some(value.clone());
                    Ok(())
                })
            }),
    ));
    let transport = Unimock::new(
        OAuthHttpTransportMock::post_form
            .next_call(matching!(_, _, _, _, _))
            .answers_arc({
                let submitted = submitted.clone();
                Arc::new(move |_, _, _, _, _, fields: &[(String, String)]| {
                    *submitted.lock().unwrap() = Some(fields.to_vec());
                    Ok(OAuthHttpResponse {
                        status: 200,
                        headers: Default::default(),
                        body: json!({
                            "access_token": "new-access",
                            "refresh_token": "new-refresh",
                            "token_type": "bearer",
                            "expires_in": 300,
                            "scope": "read write"
                        }),
                    })
                })
            }),
    );
    let oauth = refresh_manager(store, transport, clock(vec![1_000]));

    let connection = oauth.refresh(&key("account")).await.unwrap();

    let saved = saved.lock().unwrap().clone().unwrap();
    assert_eq!(saved.access_token.expose_secret(), "new-access");
    assert_eq!(saved.refresh_token.unwrap().expose_secret(), "new-refresh");
    assert_eq!(saved.expires_at, Some(1_300));
    assert_eq!(saved.scopes.as_slice(), &["read", "write"]);
    assert_eq!(connection.scopes.as_slice(), &["read", "write"]);
    let submitted = submitted.lock().unwrap().clone().unwrap();
    assert_eq!(field(&submitted, "grant_type"), "refresh_token");
    assert_eq!(field(&submitted, "refresh_token"), "old-refresh");
    assert_eq!(field(&submitted, "client_id"), "client-id");
    assert_eq!(field(&submitted, "resource"), "https://mcp.example/api");
}

#[tokio::test]
async fn refresh_retains_an_omitted_replacement_refresh_token() {
    let saved = Arc::new(Mutex::new(None::<OAuthTokenSet>));
    let old = tokens("old-access", Some("old-refresh"), Some(100));
    let store = Unimock::new((
        OAuthCredentialStoreMock::load_tokens
            .next_call(matching!(_))
            .returns(Ok(Some(old.clone()))),
        OAuthCredentialStoreMock::load_tokens
            .next_call(matching!(_))
            .returns(Ok(Some(old))),
        OAuthCredentialStoreMock::save_tokens
            .next_call(matching!(_, _))
            .answers_arc({
                let saved = saved.clone();
                Arc::new(move |_, _, value: &OAuthTokenSet| {
                    *saved.lock().unwrap() = Some(value.clone());
                    Ok(())
                })
            }),
    ));
    let transport = successful_refresh_transport("new-access");
    refresh_manager(store, transport, clock(vec![1_000]))
        .refresh(&key("account"))
        .await
        .unwrap();

    assert_eq!(
        saved
            .lock()
            .unwrap()
            .clone()
            .unwrap()
            .refresh_token
            .unwrap()
            .expose_secret(),
        "old-refresh"
    );
}

#[tokio::test]
async fn invalid_grant_deletes_tokens_and_requires_interaction() {
    let old = tokens("old-access", Some("old-refresh"), Some(100));
    let store = Unimock::new((
        OAuthCredentialStoreMock::load_tokens
            .next_call(matching!(_))
            .returns(Ok(Some(old.clone()))),
        OAuthCredentialStoreMock::load_tokens
            .next_call(matching!(_))
            .returns(Ok(Some(old))),
        OAuthCredentialStoreMock::delete_tokens
            .next_call(matching!(_))
            .returns(Ok(())),
    ));
    let transport = Unimock::new(
        OAuthHttpTransportMock::post_form
            .next_call(matching!(_, _, _, _, _))
            .returns(Ok(OAuthHttpResponse {
                status: 400,
                headers: Default::default(),
                body: json!({"error": "invalid_grant"}),
            })),
    );
    let error = refresh_manager(store, transport, Unimock::new(()))
        .refresh(&key("account"))
        .await
        .unwrap_err();

    assert!(matches!(error, Error::InteractionRequired));
}

#[tokio::test]
async fn explicit_refresh_requires_a_stored_refresh_token() {
    let current = tokens("access", None, None);
    let store = Unimock::new((
        OAuthCredentialStoreMock::load_tokens
            .next_call(matching!(_))
            .returns(Ok(Some(current.clone()))),
        OAuthCredentialStoreMock::load_tokens
            .next_call(matching!(_))
            .returns(Ok(Some(current))),
    ));

    let oauth = manager(
        Unimock::new(()),
        Unimock::new(()),
        store,
        Unimock::new(()),
        Unimock::new(()),
        Unimock::new(()),
        Unimock::new(()),
        McpOAuthConfig::default(),
    );
    let error = oauth.refresh(&key("account")).await.unwrap_err();

    assert!(matches!(error, Error::InteractionRequired));
}

#[tokio::test]
async fn transient_refresh_failure_preserves_stored_credentials() {
    let old = tokens("old-access", Some("old-refresh"), Some(100));
    let store = Unimock::new((
        OAuthCredentialStoreMock::load_tokens
            .next_call(matching!(_))
            .returns(Ok(Some(old.clone()))),
        OAuthCredentialStoreMock::load_tokens
            .next_call(matching!(_))
            .returns(Ok(Some(old))),
    ));
    let transport = Unimock::new(
        OAuthHttpTransportMock::post_form
            .next_call(matching!(_, _, _, _, _))
            .returns(Err(Error::Transport)),
    );

    let error = refresh_manager(store, transport, Unimock::new(()))
        .refresh(&key("account"))
        .await
        .unwrap_err();

    assert!(matches!(error, Error::Transport));
}

#[tokio::test]
async fn atomic_save_failure_does_not_delete_the_previous_token() {
    let old = tokens("old-access", Some("old-refresh"), Some(100));
    let store = Unimock::new((
        OAuthCredentialStoreMock::load_tokens
            .next_call(matching!(_))
            .returns(Ok(Some(old.clone()))),
        OAuthCredentialStoreMock::load_tokens
            .next_call(matching!(_))
            .returns(Ok(Some(old))),
        OAuthCredentialStoreMock::save_tokens
            .next_call(matching!(_, _))
            .returns(Err(Error::Store {
                operation: OAuthStoreOperation::SaveTokens,
            })),
    ));
    let error = refresh_manager(
        store,
        successful_refresh_transport("new-access"),
        clock(vec![1_000]),
    )
    .refresh(&key("account"))
    .await
    .unwrap_err();

    assert!(matches!(
        error,
        Error::Store {
            operation: OAuthStoreOperation::SaveTokens
        }
    ));
}

fn field<'a>(fields: &'a [(String, String)], name: &str) -> &'a str {
    fields
        .iter()
        .find(|(candidate, _)| candidate == name)
        .map(|(_, value)| value.as_str())
        .unwrap()
}
