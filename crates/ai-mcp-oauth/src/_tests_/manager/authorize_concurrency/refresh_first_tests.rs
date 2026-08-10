//! Refresh-first authorization-write interleavings.

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
    assert_authorized, authorized_response, concurrent_oauth, grant_type, invalid_grant_response,
    refreshed_response, spawn_authorize, spawn_refresh,
};

#[derive(Clone, Copy)]
enum RefreshOutcome {
    Success,
    InvalidGrant,
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn authorization_waits_for_refresh_and_wins_the_final_write() {
    assert_refresh_first_authorization_wins(RefreshOutcome::Success).await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn authorization_restores_new_tokens_after_in_flight_invalid_grant() {
    assert_refresh_first_authorization_wins(RefreshOutcome::InvalidGrant).await;
}

async fn assert_refresh_first_authorization_wins(outcome: RefreshOutcome) {
    let (authorization_saved_tx, authorization_saved_rx) = mpsc::channel();
    let (authorization_exchanged_tx, authorization_exchanged_rx) = mpsc::channel();
    let stored = Arc::new(Mutex::new(Some(tokens(
        "old-access",
        Some("old-refresh"),
        Some(100),
    ))));
    let store = stateful_store(stored.clone(), authorization_saved_tx, outcome);
    let refresh_entered = Arc::new(Barrier::new(2));
    let release_refresh = Arc::new(Barrier::new(2));
    let transport = blocking_refresh_transport(
        outcome,
        refresh_entered.clone(),
        release_refresh.clone(),
        authorization_exchanged_tx,
    );
    let oauth = concurrent_oauth(store, transport, true);

    let refresh = spawn_refresh(oauth.clone());
    refresh_entered.wait();
    let authorize = spawn_authorize(oauth.clone());
    authorization_exchanged_rx
        .recv_timeout(Duration::from_secs(1))
        .unwrap();
    let authorization_saved_early = authorization_saved_rx
        .recv_timeout(Duration::from_millis(100))
        .is_ok();
    release_refresh.wait();

    let refresh_result = refresh.await.unwrap();
    let authorization_result = authorize.await.unwrap();

    assert!(!authorization_saved_early);
    match outcome {
        RefreshOutcome::Success => assert!(refresh_result.is_ok()),
        RefreshOutcome::InvalidGrant => {
            assert!(matches!(refresh_result, Err(Error::InteractionRequired)));
        }
    }
    assert!(authorization_result.is_ok());
    assert_authorized(&stored);
}

fn stateful_store(
    stored: Arc<Mutex<Option<OAuthTokenSet>>>,
    authorization_saved: Sender<()>,
    outcome: RefreshOutcome,
) -> Unimock {
    let load_state = stored.clone();
    let save_state = stored.clone();
    let delete_state = stored;
    let load = OAuthCredentialStoreMock::load_tokens
        .each_call(matching!(_))
        .answers_arc(Arc::new(move |_, _| Ok(load_state.lock().unwrap().clone())));
    let save = OAuthCredentialStoreMock::save_tokens
        .each_call(matching!(_, _))
        .answers_arc(Arc::new(move |_, _, value: &OAuthTokenSet| {
            if value.access_token.expose_secret() == "authorized-access" {
                let _ = authorization_saved.send(());
            }
            *save_state.lock().unwrap() = Some(value.clone());
            Ok(())
        }));
    match outcome {
        RefreshOutcome::Success => Unimock::new((load, save)),
        RefreshOutcome::InvalidGrant => Unimock::new((
            load,
            save,
            OAuthCredentialStoreMock::delete_tokens
                .next_call(matching!(_))
                .answers_arc(Arc::new(move |_, _| {
                    *delete_state.lock().unwrap() = None;
                    Ok(())
                })),
        )),
    }
}

fn blocking_refresh_transport(
    outcome: RefreshOutcome,
    refresh_entered: Arc<Barrier>,
    release_refresh: Arc<Barrier>,
    authorization_exchanged: Sender<()>,
) -> Unimock {
    Unimock::new(
        OAuthHttpTransportMock::post_form
            .each_call(matching!(_, _, _, _, _))
            .answers_arc(Arc::new(move |_, _, _, _, _, fields| {
                match grant_type(fields) {
                    Some("refresh_token") => {
                        refresh_entered.wait();
                        release_refresh.wait();
                        match outcome {
                            RefreshOutcome::Success => Ok(refreshed_response()),
                            RefreshOutcome::InvalidGrant => Ok(invalid_grant_response()),
                        }
                    }
                    Some("authorization_code") => {
                        let _ = authorization_exchanged.send(());
                        Ok(authorized_response())
                    }
                    _ => Err(Error::Transport),
                }
            })),
    )
}
