//! Retry boundaries for opened streams and visible model events.

use std::{
    sync::{
        Arc, Mutex,
        atomic::{AtomicUsize, Ordering},
    },
    time::Duration,
};

use ai_interface::{ErrorKind, Model, ModelError};
use unimock::{MockFn, Unimock, matching};

use crate::{RetryingModel, SleeperMock};

use super::retry_fixtures::{config, full_random};

use super::wrapper_fixtures::{request, scripted_model, stream, success, text};

#[tokio::test]
async fn retry_reopens_before_first_event_and_shares_the_delay_budget() {
    let calls = Arc::new(AtomicUsize::new(0));
    let sleeps = Arc::new(Mutex::new(Vec::new()));
    let recorded = sleeps.clone();
    let model = RetryingModel::new(
        scripted_model(
            vec![
                Err(ModelError::transient("p", "m", "open")),
                Ok(stream(vec![Err(ModelError::transient("p", "m", "read"))])),
                Ok(success("retained")),
            ],
            calls.clone(),
        ),
        Arc::new(Unimock::new(
            SleeperMock::sleep
                .each_call(matching!(_))
                .answers_arc(Arc::new(move |_, delay| {
                    recorded.lock().unwrap().push(delay);
                })),
        )),
        full_random(),
        config(vec![Duration::from_millis(7), Duration::from_millis(11)]),
    );

    let result = model.complete(&request()).await;

    assert!(matches!(result, Ok(response) if response.text() == "retained"));
    assert_eq!(calls.load(Ordering::SeqCst), 3);
    assert_eq!(
        *sleeps.lock().unwrap(),
        vec![Duration::from_millis(7), Duration::from_millis(11)]
    );
}

#[tokio::test]
async fn retry_does_not_replay_interruptions_or_errors_after_a_visible_event() {
    for (items, interrupted) in [
        (
            vec![Err(ModelError::interrupted("p", "m", "before public text"))],
            true,
        ),
        (
            vec![
                Ok(Some(text("partial"))),
                Err(ModelError::transient("p", "m", "after text")),
            ],
            false,
        ),
    ] {
        let calls = Arc::new(AtomicUsize::new(0));
        let model = RetryingModel::new(
            scripted_model(vec![Ok(stream(items))], calls.clone()),
            Arc::new(Unimock::new(())),
            Arc::new(Unimock::new(())),
            config(vec![Duration::ZERO]),
        );

        let result = model.complete(&request()).await;

        assert!(matches!(
            (&result, interrupted),
            (Err(ModelError::Failure { core, .. }), expected)
                if core.kind == if expected { ErrorKind::Interrupted } else { ErrorKind::Transient }
        ));
        assert_eq!(calls.load(Ordering::SeqCst), 1);
    }
}

#[tokio::test]
async fn retry_exhaustion_returns_the_last_read_error_and_stays_terminal() {
    let calls = Arc::new(AtomicUsize::new(0));
    let model = RetryingModel::new(
        scripted_model(
            vec![
                Ok(stream(vec![Err(ModelError::transient("p", "m", "first"))])),
                Err(ModelError::transient("p", "m", "last")),
            ],
            calls.clone(),
        ),
        Arc::new(Unimock::new(
            SleeperMock::sleep.some_call(matching!(_)).returns(()),
        )),
        full_random(),
        config(vec![Duration::ZERO]),
    );
    let mut stream = model.stream(&request()).await.unwrap();

    let error = stream.next().await;

    assert!(
        matches!(error, Err(ModelError::Failure { core, .. }) if core.kind == ErrorKind::Transient && core.message == "last")
    );
    assert!(matches!(stream.next().await, Ok(None)));
    assert!(matches!(stream.next().await, Ok(None)));
    assert_eq!(calls.load(Ordering::SeqCst), 2);
}
