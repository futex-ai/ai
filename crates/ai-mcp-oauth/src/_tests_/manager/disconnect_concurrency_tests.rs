//! Disconnect and refresh serialization tests.

use std::{
    sync::{
        Arc, Barrier, Mutex,
        mpsc::{self, Sender},
    },
    time::Duration,
};

use serde_json::{Value, json};
use unimock::{MockFn, Unimock, matching};

use crate::{
    DefaultMcpOAuthManager, Error, McpOAuthConfig, McpOAuthDiscoveryMock, McpOAuthManager,
    OAuthClockMock, OAuthConnection, OAuthCredentialKey, OAuthCredentialStoreMock,
    OAuthHttpResponse, OAuthHttpTransportMock, OAuthTokenSet, Result,
};

use super::support::{key, manager, server_metadata, tokens};

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn disconnect_waits_for_refresh_and_removes_rotated_tokens() {
    let (deleted_tx, deleted_rx) = mpsc::channel();
    let (store, stored) = stateful_store(Some(deleted_tx));
    let refresh_entered = Arc::new(Barrier::new(2));
    let release_refresh = Arc::new(Barrier::new(2));
    let revoked = Arc::new(Mutex::new(None::<String>));
    let transport = refresh_first_transport(
        refresh_entered.clone(),
        release_refresh.clone(),
        revoked.clone(),
    );
    let oauth = concurrent_manager(
        store,
        transport,
        Unimock::new(
            OAuthClockMock::now_unix_seconds
                .next_call(matching!())
                .returns(Ok(1_000_u64)),
        ),
    );
    let credential = key("account");
    let refresh = spawn_refresh(oauth.clone(), credential.clone());
    refresh_entered.wait();
    let disconnect = spawn_disconnect(oauth.clone(), credential);

    let deleted_before_refresh_finished =
        deleted_rx.recv_timeout(Duration::from_millis(100)).is_ok();
    release_refresh.wait();
    let refresh_result = refresh.await.unwrap();
    let disconnect_result = disconnect.await.unwrap();

    assert!(!deleted_before_refresh_finished);
    assert!(refresh_result.is_ok());
    assert!(disconnect_result.is_ok());
    assert!(stored.lock().unwrap().is_none());
    assert_eq!(revoked.lock().unwrap().as_deref(), Some("new-refresh"));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn refresh_waits_for_disconnect_and_does_not_restore_tokens() {
    let (store, stored) = stateful_store_without_save();
    let revocation_entered = Arc::new(Barrier::new(2));
    let release_revocation = Arc::new(Barrier::new(2));
    let (token_tx, token_rx) = mpsc::channel();
    let transport = disconnect_first_transport(
        revocation_entered.clone(),
        release_revocation.clone(),
        token_tx,
    );
    let oauth = concurrent_manager(store, transport, Unimock::new(()));
    let credential = key("account");
    let disconnect = spawn_disconnect(oauth.clone(), credential.clone());
    revocation_entered.wait();
    let refresh = spawn_refresh(oauth.clone(), credential);

    let token_requested_before_disconnect_finished =
        token_rx.recv_timeout(Duration::from_millis(100)).is_ok();
    release_revocation.wait();
    let disconnect_result = disconnect.await.unwrap();
    let refresh_result = refresh.await.unwrap();

    assert!(!token_requested_before_disconnect_finished);
    assert!(disconnect_result.is_ok());
    assert!(matches!(refresh_result, Err(Error::InteractionRequired)));
    assert!(stored.lock().unwrap().is_none());
}

fn stateful_store(deleted: Option<Sender<()>>) -> (Unimock, Arc<Mutex<Option<OAuthTokenSet>>>) {
    let stored = Arc::new(Mutex::new(Some(tokens(
        "old-access",
        Some("old-refresh"),
        Some(100),
    ))));
    let load_state = stored.clone();
    let save_state = stored.clone();
    let delete_state = stored.clone();
    let store = Unimock::new((
        OAuthCredentialStoreMock::load_tokens
            .each_call(matching!(_))
            .answers_arc(Arc::new(move |_, _| Ok(load_state.lock().unwrap().clone()))),
        OAuthCredentialStoreMock::save_tokens
            .each_call(matching!(_, _))
            .answers_arc(Arc::new(move |_, _, value: &OAuthTokenSet| {
                *save_state.lock().unwrap() = Some(value.clone());
                Ok(())
            })),
        OAuthCredentialStoreMock::delete_tokens
            .each_call(matching!(_))
            .answers_arc(Arc::new(move |_, _| {
                *delete_state.lock().unwrap() = None;
                if let Some(deleted) = &deleted {
                    let _ = deleted.send(());
                }
                Ok(())
            })),
    ));
    (store, stored)
}

fn stateful_store_without_save() -> (Unimock, Arc<Mutex<Option<OAuthTokenSet>>>) {
    let stored = Arc::new(Mutex::new(Some(tokens(
        "old-access",
        Some("old-refresh"),
        Some(100),
    ))));
    let load_state = stored.clone();
    let delete_state = stored.clone();
    let store = Unimock::new((
        OAuthCredentialStoreMock::load_tokens
            .each_call(matching!(_))
            .answers_arc(Arc::new(move |_, _| Ok(load_state.lock().unwrap().clone()))),
        OAuthCredentialStoreMock::delete_tokens
            .next_call(matching!(_))
            .answers_arc(Arc::new(move |_, _| {
                *delete_state.lock().unwrap() = None;
                Ok(())
            })),
    ));
    (store, stored)
}

fn concurrent_manager(
    store: Unimock,
    transport: Unimock,
    clock: Unimock,
) -> Arc<DefaultMcpOAuthManager> {
    Arc::new(manager(
        Unimock::new(
            McpOAuthDiscoveryMock::authorization_server
                .each_call(matching!(_))
                .answers(&|_, _| Ok(server_metadata())),
        ),
        Unimock::new(()),
        store,
        Unimock::new(()),
        transport,
        Unimock::new(()),
        clock,
        Unimock::new(()),
        McpOAuthConfig::default(),
    ))
}

fn refresh_first_transport(
    refresh_entered: Arc<Barrier>,
    release_refresh: Arc<Barrier>,
    revoked: Arc<Mutex<Option<String>>>,
) -> Unimock {
    Unimock::new(
        OAuthHttpTransportMock::post_form
            .each_call(matching!(_, _, _, _, _))
            .answers_arc(Arc::new(move |_, url, _, _, _, fields| {
                if url.ends_with("/token") {
                    refresh_entered.wait();
                    release_refresh.wait();
                    return Ok(token_response());
                }
                *revoked.lock().unwrap() = form_value(fields, "token").map(str::to_owned);
                Ok(empty_response())
            })),
    )
}

fn disconnect_first_transport(
    revocation_entered: Arc<Barrier>,
    release_revocation: Arc<Barrier>,
    token_requested: Sender<()>,
) -> Unimock {
    Unimock::new(
        OAuthHttpTransportMock::post_form
            .each_call(matching!(_, _, _, _, _))
            .answers_arc(Arc::new(move |_, url, _, _, _, _| {
                if url.ends_with("/revoke") {
                    revocation_entered.wait();
                    release_revocation.wait();
                    return Ok(empty_response());
                }
                let _ = token_requested.send(());
                Ok(token_response())
            })),
    )
}

fn spawn_refresh(
    oauth: Arc<DefaultMcpOAuthManager>,
    credential: OAuthCredentialKey,
) -> tokio::task::JoinHandle<Result<OAuthConnection>> {
    tokio::spawn(async move { oauth.refresh(&credential).await })
}

fn spawn_disconnect(
    oauth: Arc<DefaultMcpOAuthManager>,
    credential: OAuthCredentialKey,
) -> tokio::task::JoinHandle<Result<()>> {
    tokio::spawn(async move { oauth.disconnect(&credential).await })
}

fn token_response() -> OAuthHttpResponse {
    OAuthHttpResponse {
        status: 200,
        headers: Default::default(),
        body: json!({
            "access_token": "new-access",
            "refresh_token": "new-refresh",
            "token_type": "Bearer",
            "expires_in": 300
        }),
    }
}

fn empty_response() -> OAuthHttpResponse {
    OAuthHttpResponse {
        status: 200,
        headers: Default::default(),
        body: Value::Null,
    }
}

fn form_value<'a>(fields: &'a [(String, String)], name: &str) -> Option<&'a str> {
    fields
        .iter()
        .find(|(candidate, _)| candidate == name)
        .map(|(_, value)| value.as_str())
}
