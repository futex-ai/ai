//! Deterministic jitter bounds and retry-after limits.

use std::{
    error::Error,
    sync::{
        Arc, Mutex,
        atomic::{AtomicUsize, Ordering},
    },
    time::Duration,
};

use ai_interface::{ErrorCore, ErrorKind, Model, ModelError, SafeErrorSource};
use unimock::{MockFn, Unimock, matching};

use crate::{RetryConfig, RetryRandomMock, RetryingModel};

use super::{
    retry_fixtures::{config, full_random, recording_sleeper},
    wrapper_fixtures::{request, scripted_model, stream, success},
};

#[tokio::test]
async fn jitter_uses_inclusive_half_to_full_bounds_and_the_larger_retry_after() {
    for (jitter, retry_after, expected) in [
        (50, None, 50),
        (100, None, 100),
        (75, Some(20), 75),
        (50, Some(175), 175),
        (100, Some(300), 300),
    ] {
        let mut error = ModelError::transient("p", "m", "try again");
        error.core_mut().retry_after = retry_after.map(Duration::from_millis);
        let waits = Arc::new(Mutex::new(Vec::new()));
        let bounds = Arc::new(Mutex::new(Vec::new()));
        let recorded = bounds.clone();
        let random = Arc::new(Unimock::new(
            RetryRandomMock::between
                .some_call(matching!(_, _))
                .answers_arc(Arc::new(move |_, minimum, maximum| {
                    recorded.lock().unwrap().push((minimum, maximum));
                    Duration::from_millis(jitter)
                })),
        ));
        let mut config = config(vec![Duration::from_millis(100)]);
        config.max_wait = Duration::from_millis(300);
        let model = RetryingModel::new(
            scripted_model(
                vec![Err(error), Ok(success("done"))],
                Arc::new(AtomicUsize::new(0)),
            ),
            recording_sleeper(waits.clone()),
            random,
            config,
        );
        let result = model.complete(&request()).await;
        assert_eq!(
            result.as_ref().map(|r| r.text()).map_err(|e| e.core().kind),
            Ok("done".to_owned())
        );
        assert_eq!(
            *bounds.lock().unwrap(),
            vec![(Duration::from_millis(50), Duration::from_millis(100))]
        );
        assert_eq!(
            *waits.lock().unwrap(),
            vec![Duration::from_millis(expected)]
        );
    }
}

#[tokio::test]
async fn retry_after_cap_returns_the_original_error_and_preserves_its_source_and_metadata() {
    for (retry_after, maximum, retries) in [(30_000, 30_000, 1), (30_001, 30_000, 0), (8, 7, 0)] {
        let mut core = ErrorCore::new(ErrorKind::Transient, "provider", "model", "safe original");
        core.retry_after = Some(Duration::from_millis(retry_after));
        core.provider_request_id = Some("req-123".to_owned());
        let error = ModelError::with_source(
            core.clone(),
            SafeErrorSource::redacted(std::io::Error::other("source")),
        );
        let calls = Arc::new(AtomicUsize::new(0));
        let waits = Arc::new(Mutex::new(Vec::new()));
        let mut settings = RetryConfig {
            base_delays: vec![Duration::ZERO],
            ..RetryConfig::default()
        };
        if maximum != 30_000 {
            settings.max_wait = Duration::from_millis(maximum);
        }
        let mut attempts = vec![Err(error)];
        if retries == 1 {
            attempts.push(Ok(stream(vec![Err(ModelError::with_source(
                core.clone(),
                SafeErrorSource::redacted(std::io::Error::other("source")),
            ))])));
        }
        let model = RetryingModel::new(
            scripted_model(attempts, calls.clone()),
            if retries == 0 {
                Arc::new(Unimock::new(()))
            } else {
                recording_sleeper(waits.clone())
            },
            if retries == 0 {
                Arc::new(Unimock::new(()))
            } else {
                full_random()
            },
            settings,
        );
        let error = model.complete(&request()).await.unwrap_err();
        assert_eq!(calls.load(Ordering::SeqCst), 1 + retries);
        assert_eq!(error.core(), &core);
        assert_eq!(
            error
                .source()
                .unwrap()
                .downcast_ref::<SafeErrorSource>()
                .unwrap()
                .raw_cause()
                .downcast_ref::<std::io::Error>()
                .unwrap()
                .to_string(),
            "source"
        );
        assert_eq!(
            *waits.lock().unwrap(),
            if retries == 1 {
                vec![Duration::from_millis(30_000)]
            } else {
                vec![]
            }
        );
    }
}
