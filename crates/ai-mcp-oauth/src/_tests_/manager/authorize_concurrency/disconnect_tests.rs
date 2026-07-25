//! Authorization and disconnect interleaving.

use std::{
    sync::{Arc, Barrier, Mutex},
    time::Duration,
};

use serde_json::Value;
use unimock::{MockFn, Unimock, matching};

use crate::{
    DefaultMcpOAuthManager, McpOAuthManager, OAuthCredentialKey, OAuthCredentialStoreMock,
    OAuthHttpResponse, OAuthHttpTransportMock, OAuthTokenSet, Result,
};

use super::super::support::{key, tokens};
use super::support::{
    assert_authorized, authorized_response, concurrent_oauth, grant_type, spawn_authorize,
};

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn authorization_finishing_after_disconnect_stores_the_new_grant() {
    let exchange_entered = Arc::new(Barrier::new(2));
    let release_exchange = Arc::new(Barrier::new(2));
    let stored = Arc::new(Mutex::new(Some(tokens(
        "old-access",
        Some("old-refresh"),
        Some(100),
    ))));
    let store = stateful_store(stored.clone());
    let transport = blocking_exchange_transport(exchange_entered.clone(), release_exchange.clone());
    let oauth = concurrent_oauth(store, transport, true);

    let authorize = spawn_authorize(oauth.clone());
    exchange_entered.wait();
    let disconnect = spawn_disconnect(oauth.clone(), key("account"));
    let disconnect_result = tokio::time::timeout(Duration::from_secs(1), disconnect)
        .await
        .unwrap()
        .unwrap();
    assert!(disconnect_result.is_ok());
    assert!(stored.lock().unwrap().is_none());

    release_exchange.wait();
    assert!(authorize.await.unwrap().is_ok());
    assert_authorized(&stored);
}

fn stateful_store(stored: Arc<Mutex<Option<OAuthTokenSet>>>) -> Unimock {
    let load_state = stored.clone();
    let delete_state = stored.clone();
    let save_state = stored;
    Unimock::new((
        OAuthCredentialStoreMock::load_tokens
            .next_call(matching!(_))
            .answers_arc(Arc::new(move |_, _| Ok(load_state.lock().unwrap().clone()))),
        OAuthCredentialStoreMock::delete_tokens
            .next_call(matching!(_))
            .answers_arc(Arc::new(move |_, _| {
                *delete_state.lock().unwrap() = None;
                Ok(())
            })),
        OAuthCredentialStoreMock::save_tokens
            .next_call(matching!(_, _))
            .answers_arc(Arc::new(move |_, _, value: &OAuthTokenSet| {
                *save_state.lock().unwrap() = Some(value.clone());
                Ok(())
            })),
    ))
}

fn blocking_exchange_transport(
    exchange_entered: Arc<Barrier>,
    release_exchange: Arc<Barrier>,
) -> Unimock {
    Unimock::new(
        OAuthHttpTransportMock::post_form
            .each_call(matching!(_, _, _, _, _))
            .answers_arc(Arc::new(move |_, _, _, _, _, fields| {
                if grant_type(fields) == Some("authorization_code") {
                    exchange_entered.wait();
                    release_exchange.wait();
                    return Ok(authorized_response());
                }
                Ok(OAuthHttpResponse {
                    status: 200,
                    headers: Default::default(),
                    body: Value::Null,
                })
            })),
    )
}

fn spawn_disconnect(
    oauth: Arc<DefaultMcpOAuthManager>,
    credential: OAuthCredentialKey,
) -> tokio::task::JoinHandle<Result<()>> {
    tokio::spawn(async move { oauth.disconnect(&credential).await })
}
