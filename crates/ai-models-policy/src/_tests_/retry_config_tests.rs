//! Shared attempt limits and unchanged complete requests.

use std::{
    num::NonZeroUsize,
    sync::{
        Arc, Mutex,
        atomic::{AtomicUsize, Ordering},
    },
    time::Duration,
};

use ai_interface::{ErrorKind, Model, ModelError, ModelMock, ModelRequest};
use unimock::{MockFn, Unimock, matching};

use crate::{RetryConfig, RetryingModel};

use super::{
    retry_fixtures::{config, full_random, recording_sleeper},
    wrapper_fixtures::{request, scripted_model, stream, success},
};

#[tokio::test]
async fn attempt_count_and_schedule_exhaustion_bound_retries_across_open_and_read() {
    for (settings, expected_calls, expected_waits) in [
        (RetryConfig::default(), 3, vec![100, 250]),
        (
            RetryConfig {
                base_delays: vec![
                    Duration::from_millis(100),
                    Duration::from_millis(250),
                    Duration::from_millis(500),
                ],
                ..RetryConfig::default()
            },
            3,
            vec![100, 250],
        ),
        (
            RetryConfig {
                max_attempts: NonZeroUsize::new(1).unwrap(),
                ..config(vec![Duration::ZERO; 3])
            },
            1,
            vec![],
        ),
        (
            RetryConfig {
                max_attempts: NonZeroUsize::new(4).unwrap(),
                ..config(vec![Duration::from_millis(7)])
            },
            2,
            vec![7],
        ),
        (config(vec![Duration::ZERO; 3]), 4, vec![0, 0, 0]),
    ] {
        let calls = Arc::new(AtomicUsize::new(0));
        let waits = Arc::new(Mutex::new(Vec::new()));
        let attempts = (0..expected_calls)
            .map(|index| {
                let error = ModelError::transient("p", "m", "original");
                if index % 2 == 0 {
                    Err(error)
                } else {
                    Ok(stream(vec![Err(error)]))
                }
            })
            .collect();
        let model = RetryingModel::new(
            scripted_model(attempts, calls.clone()),
            if expected_calls == 1 {
                Arc::new(Unimock::new(()))
            } else {
                recording_sleeper(waits.clone())
            },
            if expected_calls == 1 {
                Arc::new(Unimock::new(()))
            } else {
                full_random()
            },
            settings,
        );
        let result = model.complete(&request()).await;
        assert_eq!(result.unwrap_err().core().kind, ErrorKind::Transient);
        assert_eq!(calls.load(Ordering::SeqCst), expected_calls);
        assert_eq!(
            *waits.lock().unwrap(),
            expected_waits
                .into_iter()
                .map(Duration::from_millis)
                .collect::<Vec<_>>()
        );
    }
}

#[tokio::test]
async fn every_retry_receives_the_full_original_request_and_total_timeout() {
    let original = super::request_fixtures::controlled_request();
    let seen = Arc::new(Mutex::new(Vec::new()));
    let recorded = seen.clone();
    let inner = Arc::new(Unimock::new(
        ModelMock::stream
            .each_call(matching!(_))
            .answers_arc(Arc::new(move |_, received: &ModelRequest| {
                let mut seen = recorded.lock().unwrap();
                seen.push(received.clone());
                if seen.len() < 3 {
                    Err(ModelError::transient("p", "m", "again"))
                } else {
                    Ok(success("done"))
                }
            })),
    ));
    let model = RetryingModel::new(
        inner,
        recording_sleeper(Arc::new(Mutex::new(Vec::new()))),
        full_random(),
        config(vec![Duration::from_secs(3); 2]),
    );
    let result = model.complete(&original).await;
    assert_eq!(
        result.as_ref().map(|r| r.text()).map_err(|e| e.core().kind),
        Ok("done".to_owned())
    );
    assert_eq!(
        *seen.lock().unwrap(),
        vec![original.clone(), original.clone(), original]
    );
}
