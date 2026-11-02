//! Exact retry categories and first-event replay boundaries.

use std::{
    sync::{
        Arc, Mutex,
        atomic::{AtomicUsize, Ordering},
    },
    time::Duration,
};

use ai_interface::{ErrorKind, Model, ModelError, ModelEvent};

use unimock::Unimock;

use crate::{DynRetryRandom, DynSleeper, RetryingModel};

use super::{
    retry_fixtures::{KINDS, config, full_random, recording_sleeper},
    wrapper_fixtures::{request, scripted_model, stream, success, text},
};

#[tokio::test]
async fn retries_only_rate_limited_and_transient_kinds_on_open_and_pre_event_read() {
    for (kind, _, retry) in KINDS {
        for reading in [false, true] {
            let failure = ModelError::failure(kind, "provider", "model", "original");
            let attempt = if reading {
                Ok(stream(vec![Err(failure)]))
            } else {
                Err(failure)
            };
            let calls = Arc::new(AtomicUsize::new(0));
            let waits = Arc::new(Mutex::new(Vec::new()));
            let sleeper: DynSleeper = if retry {
                recording_sleeper(waits.clone())
            } else {
                Arc::new(Unimock::new(()))
            };
            let random: DynRetryRandom = if retry {
                full_random()
            } else {
                Arc::new(Unimock::new(()))
            };
            let model = RetryingModel::new(
                scripted_model(vec![attempt, Ok(success("winner"))], calls.clone()),
                sleeper,
                random,
                config(vec![Duration::from_millis(100)]),
            );
            let actual = model.complete(&request()).await;
            assert_eq!(
                actual.as_ref().map(|r| r.text()).map_err(|e| e.core().kind),
                if retry {
                    Ok("winner".to_owned())
                } else {
                    Err(kind)
                },
                "{kind:?}, reading={reading}"
            );
            assert_eq!(calls.load(Ordering::SeqCst), if retry { 2 } else { 1 });
            assert_eq!(
                *waits.lock().unwrap(),
                if retry {
                    vec![Duration::from_millis(100)]
                } else {
                    vec![]
                }
            );
        }
    }
}

#[tokio::test]
async fn retry_stops_after_any_first_event_including_an_inner_fallback() {
    for event in [
        None,
        Some(text("visible")),
        Some(ModelEvent::Fallback {
            error: ModelError::invalid_request("inner", "earlier", "failed"),
            emitted_events: false,
        }),
    ] {
        let visible = event.is_some();
        let calls = Arc::new(AtomicUsize::new(0));
        let mut items = Vec::new();
        if let Some(event) = event {
            items.push(Ok(Some(event)));
        }
        items.push(Err(ModelError::transient("p", "m", "read")));
        let model = RetryingModel::new(
            scripted_model(
                vec![Ok(stream(items)), Ok(success("winner"))],
                calls.clone(),
            ),
            if visible {
                Arc::new(Unimock::new(()))
            } else {
                recording_sleeper(Arc::new(Mutex::new(Vec::new())))
            },
            if visible {
                Arc::new(Unimock::new(()))
            } else {
                full_random()
            },
            config(vec![Duration::ZERO]),
        );
        let result = model.complete(&request()).await;
        assert_eq!(
            result.as_ref().map(|r| r.text()).map_err(|e| e.core().kind),
            if visible {
                Err(ErrorKind::Transient)
            } else {
                Ok("winner".to_owned())
            }
        );
        assert_eq!(calls.load(Ordering::SeqCst), if visible { 1 } else { 2 });
    }
}
