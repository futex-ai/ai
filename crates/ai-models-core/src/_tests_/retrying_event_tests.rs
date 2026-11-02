//! Stream event forwarding through transient retry policy.

use std::{
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    },
    time::Duration,
};

use ai_interface::{ErrorKind, ModelError, ModelRequest};
use serde_json::{Value, json};
use unimock::{MockFn, Unimock, matching};

use crate::{RetryingModel, SleeperMock, test_support::collect_model_events};

use super::wrapper_fixtures::{scripted_model, stream, success, text};

#[tokio::test]
async fn transient_retry_emits_only_the_successful_attempt_events() {
    let calls = Arc::new(AtomicUsize::new(0));
    let model = RetryingModel::new(
        scripted_model(
            vec![
                Err(ModelError::transient("openai", "gpt", "retry")),
                Ok(success("ok")),
            ],
            calls.clone(),
        ),
        Arc::new(Unimock::new(
            SleeperMock::sleep.next_call(matching!(_)).returns(()),
        )),
        vec![Duration::ZERO],
    );
    let (response, events) = collect_model_events(&model, &empty_request()).await;
    let response = response.expect("second attempt should succeed");

    assert_eq!(response.text(), "ok");
    assert_eq!(calls.load(Ordering::SeqCst), 2);
    assert_eq!(recorded_events(events), vec![assistant_delta("ok")]);
}

#[tokio::test]
async fn interrupted_event_attempt_is_not_retried() {
    let calls = Arc::new(AtomicUsize::new(0));
    let model = RetryingModel::new(
        scripted_model(
            vec![Ok(stream(vec![
                Ok(Some(text("partial"))),
                Err(ModelError::interrupted("openai", "gpt", "stream closed")),
            ]))],
            calls.clone(),
        ),
        Arc::new(Unimock::new(())),
        Vec::new(),
    );
    let (result, events) = collect_model_events(&model, &empty_request()).await;
    let error = result.expect_err("interruption should be returned immediately");

    assert_eq!(error.core().kind, ErrorKind::Interrupted);
    assert_eq!(calls.load(Ordering::SeqCst), 1);
    assert_eq!(recorded_events(events), vec![assistant_delta("partial")]);
}

fn recorded_events(events: Vec<Value>) -> Vec<Value> {
    events
        .into_iter()
        .filter(|event| event.get("Usage").is_none() && event.get("Finish").is_none())
        .collect()
}

fn assistant_delta(delta: &str) -> Value {
    json!({"TextDelta": {"part_index": 0, "delta": delta}})
}

fn empty_request() -> ModelRequest {
    ModelRequest {
        system_prompt: "system".to_owned(),
        messages: Vec::new(),
        tools: Vec::new(),
        response_schema: None,
        controls: Default::default(),
    }
}
