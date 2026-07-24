//! Auth-hook refresh behavior and credential-lock concurrency tests.

use std::{
    sync::{
        Arc, Barrier, Mutex,
        atomic::{AtomicUsize, Ordering},
    },
    time::Duration,
};

use secrecy::ExposeSecret;
use serde_json::json;
use unimock::{MockFn, Unimock, matching};

use crate::{
    Error, McpOAuthConfig, OAuthClockMock, OAuthCredentialStoreMock, OAuthHttpResponse,
    OAuthHttpTransportMock, OAuthRequestTokenProvider, OAuthTokenSet,
};

use super::support::{key, manager, refresh_manager, tokens};

#[tokio::test]
async fn auth_hook_invalid_grant_clears_credentials_and_returns_no_token() {
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
    let repeated_clock = Unimock::new(
        OAuthClockMock::now_unix_seconds
            .each_call(matching!())
            .answers(&|_| Ok(200)),
    );

    let token = refresh_manager(store, transport, repeated_clock)
        .token_for_request(&key("account"))
        .await
        .unwrap();

    assert!(token.is_none());
}

#[tokio::test]
async fn auth_hook_transient_failure_does_not_send_an_expired_token() {
    let old = tokens("expired-access", Some("refresh"), Some(100));
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
    let repeated_clock = Unimock::new(
        OAuthClockMock::now_unix_seconds
            .each_call(matching!())
            .answers(&|_| Ok(200)),
    );

    let error = refresh_manager(store, transport, repeated_clock)
        .token_for_request(&key("account"))
        .await
        .unwrap_err();

    assert!(matches!(error, Error::Transport));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn same_key_concurrent_callers_share_one_refresh() {
    let current = Arc::new(Mutex::new(tokens(
        "old-access",
        Some("old-refresh"),
        Some(100),
    )));
    let initial_barrier = Arc::new(Barrier::new(2));
    let loads = Arc::new(AtomicUsize::new(0));
    let store = Unimock::new((
        OAuthCredentialStoreMock::load_tokens
            .each_call(matching!(_))
            .answers_arc({
                let current = current.clone();
                let initial_barrier = initial_barrier.clone();
                let loads = loads.clone();
                Arc::new(move |_, _| {
                    let call = loads.fetch_add(1, Ordering::SeqCst);
                    let snapshot = current.lock().unwrap().clone();
                    if call < 2 {
                        initial_barrier.wait();
                    }
                    Ok(Some(snapshot))
                })
            }),
        OAuthCredentialStoreMock::save_tokens
            .next_call(matching!(_, _))
            .answers_arc({
                let current = current.clone();
                Arc::new(move |_, _, value: &OAuthTokenSet| {
                    *current.lock().unwrap() = value.clone();
                    Ok(())
                })
            }),
    ));
    let refresh_calls = Arc::new(AtomicUsize::new(0));
    let transport = Unimock::new(
        OAuthHttpTransportMock::post_form
            .next_call(matching!(_, _, _, _, _))
            .answers_arc({
                let refresh_calls = refresh_calls.clone();
                Arc::new(move |_, _, _, _, _, _| {
                    refresh_calls.fetch_add(1, Ordering::SeqCst);
                    Ok(OAuthHttpResponse {
                        status: 200,
                        headers: Default::default(),
                        body: json!({
                            "access_token": "old-access",
                            "refresh_token": "old-refresh",
                            "token_type": "Bearer",
                            "expires_in": 30
                        }),
                    })
                })
            }),
    );
    let repeated_clock = Unimock::new(
        OAuthClockMock::now_unix_seconds
            .each_call(matching!())
            .answers(&|_| Ok(200)),
    );
    let oauth = Arc::new(refresh_manager(store, transport, repeated_clock));
    let credential = key("account");
    let left_oauth = oauth.clone();
    let left_key = credential.clone();
    let left = tokio::spawn(async move { left_oauth.token_for_request(&left_key).await });
    let right_oauth = oauth.clone();
    let right = tokio::spawn(async move { right_oauth.token_for_request(&credential).await });

    assert_eq!(
        left.await.unwrap().unwrap().unwrap().expose_secret(),
        "old-access"
    );
    assert_eq!(
        right.await.unwrap().unwrap().unwrap().expose_secret(),
        "old-access"
    );
    assert_eq!(refresh_calls.load(Ordering::SeqCst), 1);
}

#[tokio::test]
async fn unrelated_credential_locks_do_not_block_each_other() {
    let oauth = manager(
        Unimock::new(()),
        Unimock::new(()),
        Unimock::new(()),
        Unimock::new(()),
        Unimock::new(()),
        Unimock::new(()),
        Unimock::new(()),
        McpOAuthConfig::default(),
    );
    let left = oauth.refresh_lock(&key("left")).await;
    let right = oauth.refresh_lock(&key("right")).await;
    let _left_guard = left.lock().await;

    let right_guard = tokio::time::timeout(Duration::from_millis(20), right.lock()).await;

    assert!(right_guard.is_ok());
}
