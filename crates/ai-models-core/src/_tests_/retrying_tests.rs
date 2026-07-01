//! Tests for transient retry scheduling.

use std::{
    collections::VecDeque,
    sync::{Arc, Mutex},
    time::Duration,
};

use ai_interface::{
    FinishReason, Model, ModelError, ModelMock, ModelRequest, ModelResponse, ModelUsage,
};
use unimock::{MockFn, Unimock, matching};

use crate::{RetryingModel, Sleeper, SleeperMock};

#[tokio::test]
async fn retries_transient_failures_using_the_configured_schedule() {
    let sleeps = Arc::new(Mutex::new(Vec::new()));
    let retrying = RetryingModel::new(
        scripted_model(vec![
            Err(ModelError::transient_provider(
                "openai", "gpt-5.5", "retry 1",
            )),
            Err(ModelError::transient_provider(
                "openai", "gpt-5.5", "retry 2",
            )),
            Ok(success_response()),
        ]),
        recording_sleeper(sleeps.clone()),
        vec![Duration::from_millis(100), Duration::from_millis(250)],
    );

    let response = retrying
        .complete(&empty_request())
        .await
        .expect("third attempt should succeed");

    assert_eq!(response.assistant_message, "ok");
    assert_eq!(
        *sleeps.lock().expect("sleep lock should not be poisoned"),
        vec![Duration::from_millis(100), Duration::from_millis(250)]
    );
}

#[tokio::test]
async fn returns_the_last_transient_error_after_exhausting_retries() {
    let retrying = RetryingModel::new(
        scripted_model(vec![
            Err(ModelError::transient_provider(
                "openai", "gpt-5.5", "retry 1",
            )),
            Err(ModelError::transient_provider(
                "openai", "gpt-5.5", "retry 2",
            )),
            Err(ModelError::transient_provider(
                "openai", "gpt-5.5", "retry 3",
            )),
        ]),
        recording_sleeper(Arc::new(Mutex::new(Vec::new()))),
        vec![Duration::from_millis(100), Duration::from_millis(250)],
    );

    let error = retrying
        .complete(&empty_request())
        .await
        .expect_err("third transient failure should be returned");

    assert!(matches!(error, ModelError::TransientProvider { .. }));
}

fn scripted_model(
    responses: Vec<std::result::Result<ModelResponse, ModelError>>,
) -> Arc<dyn Model> {
    let responses = Arc::new(Mutex::new(VecDeque::from(responses)));
    Arc::new(Unimock::new(
        ModelMock::complete.each_call(matching!(_)).answers_arc({
            let responses = responses.clone();
            Arc::new(move |_, _request: &ModelRequest| {
                responses
                    .lock()
                    .expect("responses lock should not be poisoned")
                    .pop_front()
                    .expect("unexpected model call")
            })
        }),
    ))
}

fn recording_sleeper(sleeps: Arc<Mutex<Vec<Duration>>>) -> Arc<dyn Sleeper> {
    Arc::new(Unimock::new(
        SleeperMock::sleep.each_call(matching!(_)).answers_arc({
            let sleeps = sleeps.clone();
            Arc::new(move |_, duration: Duration| {
                sleeps
                    .lock()
                    .expect("sleep lock should not be poisoned")
                    .push(duration);
            })
        }),
    ))
}

fn empty_request() -> ModelRequest {
    ModelRequest {
        system_prompt: "system".to_owned(),
        messages: Vec::new(),
        tools: Vec::new(),
        response_schema: None,
    }
}

fn success_response() -> ModelResponse {
    ModelResponse {
        provider: "openai".to_owned(),
        model_id: "gpt-5.5".to_owned(),
        catalog_model_id: None,
        thinking_level: None,
        assistant_message: "ok".to_owned(),
        tool_calls: Vec::new(),
        finish_reason: FinishReason::Stop,
        structured_output: None,
        provider_context: Vec::new(),
        usage: ModelUsage::default(),
    }
}
