//! Authorization-first refresh interleaving.

use std::{
    sync::{
        Arc, Barrier, Mutex,
        mpsc::{self, Sender},
    },
    time::Duration,
};

use secrecy::ExposeSecret;
use unimock::{MockFn, Unimock, matching};

use crate::{Error, OAuthCredentialStoreMock, OAuthHttpTransportMock, OAuthTokenSet};

use super::super::support::tokens;
use super::support::{
    assert_authorized, authorized_response, concurrent_oauth, grant_type, refreshed_response,
    spawn_authorize, spawn_refresh,
};

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn refresh_waits_for_authorization_and_reuses_the_new_grant() {
    let (refresh_loaded_tx, refresh_loaded_rx) = mpsc::channel();
    let (refresh_requested_tx, refresh_requested_rx) = mpsc::channel();
    let authorization_save_entered = Arc::new(Barrier::new(2));
    let release_authorization_save = Arc::new(Barrier::new(2));
    let stored = Arc::new(Mutex::new(Some(tokens(
        "old-access",
        Some("old-refresh"),
        Some(100),
    ))));
    let store = blocking_authorization_store(
        stored.clone(),
        refresh_loaded_tx,
        authorization_save_entered.clone(),
        release_authorization_save.clone(),
    );
    let transport = routed_transport(refresh_requested_tx);
    let oauth = concurrent_oauth(store, transport, false);

    let authorize = spawn_authorize(oauth.clone());
    authorization_save_entered.wait();
    let refresh = spawn_refresh(oauth.clone());
    refresh_loaded_rx
        .recv_timeout(Duration::from_secs(1))
        .unwrap();
    let refresh_requested_early = refresh_requested_rx
        .recv_timeout(Duration::from_millis(100))
        .is_ok();
    release_authorization_save.wait();

    let authorization_result = authorize.await.unwrap();
    let refresh_result = refresh.await.unwrap().unwrap();

    assert!(!refresh_requested_early);
    assert!(authorization_result.is_ok());
    assert_eq!(refresh_result.scopes.as_slice(), &["read", "write"]);
    assert_authorized(&stored);
}

fn blocking_authorization_store(
    stored: Arc<Mutex<Option<OAuthTokenSet>>>,
    refresh_loaded: Sender<()>,
    authorization_save_entered: Arc<Barrier>,
    release_authorization_save: Arc<Barrier>,
) -> Unimock {
    let load_state = stored.clone();
    let save_state = stored;
    Unimock::new((
        OAuthCredentialStoreMock::load_tokens
            .each_call(matching!(_))
            .answers_arc(Arc::new(move |_, _| {
                let _ = refresh_loaded.send(());
                Ok(load_state.lock().unwrap().clone())
            })),
        OAuthCredentialStoreMock::save_tokens
            .each_call(matching!(_, _))
            .answers_arc(Arc::new(move |_, _, value: &OAuthTokenSet| {
                if value.access_token.expose_secret() == "authorized-access" {
                    authorization_save_entered.wait();
                    release_authorization_save.wait();
                }
                *save_state.lock().unwrap() = Some(value.clone());
                Ok(())
            })),
    ))
}

fn routed_transport(refresh_requested: Sender<()>) -> Unimock {
    Unimock::new(
        OAuthHttpTransportMock::post_form
            .each_call(matching!(_, _, _, _, _))
            .answers_arc(Arc::new(move |_, _, _, _, _, fields| {
                match grant_type(fields) {
                    Some("authorization_code") => Ok(authorized_response()),
                    Some("refresh_token") => {
                        let _ = refresh_requested.send(());
                        Ok(refreshed_response())
                    }
                    _ => Err(Error::Transport),
                }
            })),
    )
}
