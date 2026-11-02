//! Retry safety when a composed model reports earlier failed leaves.

use std::{
    slice,
    sync::{
        Arc, Mutex,
        atomic::{AtomicUsize, Ordering},
    },
    time::Duration,
};

use ai_interface::{
    DynModelEventStream, ErrorCore, ErrorKind, Model, ModelAttempt, ModelError, ModelResult,
};

use crate::{AnyError, FallbackModel, RetryingModel};

use super::{
    request_fixtures::{capturing_model, controlled_request},
    retry_fixtures::{config, full_random, recording_sleeper},
};

#[tokio::test]
async fn retry_over_fallback_does_not_replay_interrupted_leaves_and_keeps_transient_exceptions_retryable()
 {
    let waits = Arc::new(Mutex::new(Vec::new()));
    let sleeper = recording_sleeper(waits.clone());
    let random = full_random();
    for (kind, provider, message, expected_calls, expected_waits) in [
        (
            ErrorKind::Interrupted,
            "first",
            "stream failed after progress",
            1,
            0,
        ),
        (
            ErrorKind::Transient,
            "xai",
            "deferred timeout after accepted job",
            2,
            1,
        ),
        (
            ErrorKind::Transient,
            "deepseek",
            "insufficient_system_resource",
            2,
            2,
        ),
    ] {
        let original = controlled_request();
        let requests = Arc::new(Mutex::new(Vec::new()));
        let first_calls = Arc::new(AtomicUsize::new(0));
        let last_calls = Arc::new(AtomicUsize::new(0));
        let first = ErrorCore::new(kind, provider, "first-model", message);
        let mut last = ErrorCore::new(
            ErrorKind::Transient,
            "second",
            "last-model",
            "temporary failure",
        );
        last.provider_request_id = Some("request-123".to_owned());
        let fallback = FallbackModel::new(
            vec![
                capturing_model(failures(&first, &[]), requests.clone(), first_calls.clone()),
                capturing_model(failures(&last, &[]), requests.clone(), last_calls.clone()),
            ],
            Arc::new(AnyError),
        );
        let retry = RetryingModel::new(
            Arc::new(fallback),
            sleeper.clone(),
            random.clone(),
            config(vec![Duration::from_millis(7)]),
        );
        let result = retry.complete(&original).await;

        assert_eq!(
            first_calls.load(Ordering::SeqCst),
            expected_calls,
            "earlier kind: {kind:?}"
        );
        assert_eq!(last_calls.load(Ordering::SeqCst), expected_calls);
        let error = result.unwrap_err();
        assert_eq!(error.core(), &last);
        assert_eq!(
            error.attempts(),
            &[ModelAttempt {
                provider: provider.to_owned(),
                model_id: "first-model".to_owned(),
                kind,
                message: message.to_owned(),
                emitted_events: false,
            }]
        );
        assert_eq!(
            *requests.lock().unwrap(),
            vec![original; expected_calls * 2]
        );
        assert_eq!(
            *waits.lock().unwrap(),
            vec![Duration::from_millis(7); expected_waits]
        );
    }
}

#[tokio::test]
async fn retry_does_not_replay_an_earlier_attempt_that_emitted_events() {
    let waits = Arc::new(Mutex::new(Vec::new()));
    let sleeper = recording_sleeper(waits.clone());
    let random = full_random();
    for (emitted_events, expected_calls, expected_waits) in [(true, 1, 0), (false, 2, 1)] {
        let original = controlled_request();
        let requests = Arc::new(Mutex::new(Vec::new()));
        let calls = Arc::new(AtomicUsize::new(0));
        let core = ErrorCore::new(ErrorKind::Transient, "last", "model", "temporary failure");
        let earlier = ModelAttempt {
            provider: "earlier".to_owned(),
            model_id: "model".to_owned(),
            kind: ErrorKind::Transient,
            message: "earlier failure".to_owned(),
            emitted_events,
        };
        let retry = RetryingModel::new(
            capturing_model(
                failures(&core, slice::from_ref(&earlier)),
                requests.clone(),
                calls.clone(),
            ),
            sleeper.clone(),
            random.clone(),
            config(vec![Duration::from_millis(7)]),
        );
        let result = retry.complete(&original).await;

        assert_eq!(
            calls.load(Ordering::SeqCst),
            expected_calls,
            "earlier visibility: {emitted_events}"
        );
        let error = result.unwrap_err();
        assert_eq!(error.core(), &core);
        assert_eq!(error.attempts(), &[earlier]);
        assert_eq!(*requests.lock().unwrap(), vec![original; expected_calls]);
        assert_eq!(
            *waits.lock().unwrap(),
            vec![Duration::from_millis(7); expected_waits]
        );
    }
}

fn failures(core: &ErrorCore, earlier: &[ModelAttempt]) -> Vec<ModelResult<DynModelEventStream>> {
    (0..2)
        .map(|_| Err(ModelError::new(core.clone()).with_attempts(earlier.to_vec())))
        .collect()
}
