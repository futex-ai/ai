//! Probe-marker retry decision and runner regression tests.

use std::{
    collections::VecDeque,
    sync::{
        Arc, Mutex,
        atomic::{AtomicUsize, Ordering},
    },
};

use ai_interface::{
    DynModel, DynModelEventStream, Effort, FinishReason, MockModel, Model, ModelCompletionMode,
    ModelRequest, ModelResponse, ModelResult, ModelSpec, ModelToolChoice, ModelUsage,
};

use ai_interface::test_support::model_response_stream;
use async_trait::async_trait;

use super::{
    provider_tests::LiveProvider,
    runner_tests::{
        MODEL_TIMEOUT, complete_through_runtime, probe_controls, run_probe_with_marker_retry,
        should_retry_marker_miss,
    },
};

const MARKER_FAILURE: &str = "model: assistant response did not contain the probe marker";

#[test]
fn marker_only_failure_is_retryable() {
    assert!(should_retry_marker_miss(&[MARKER_FAILURE.to_owned()], &[]));
    assert!(should_retry_marker_miss(
        &[MARKER_FAILURE.to_owned()],
        &["model: assistant events did not have terminal parity".to_owned()],
    ));
    assert!(!should_retry_marker_miss(&[], &[]));
    assert!(!should_retry_marker_miss(
        &[MARKER_FAILURE.to_owned(), MARKER_FAILURE.to_owned()],
        &[],
    ));
    assert!(!should_retry_marker_miss(
        &["model: catalog model was None".to_owned()],
        &[],
    ));
}

#[test]
fn response_contract_failures_are_not_retryable() {
    for other_failure in [
        "model: provider was `wrong`, expected `minimax`",
        "model: provider model was `wrong`, expected `MiniMax-M3`",
        "model: catalog model was None",
        "model: effort was None, expected `medium`",
        "model: finish reason was Truncated",
        "model: response unexpectedly requested tools",
        "model: provider reported zero total tokens",
    ] {
        assert!(!should_retry_marker_miss(
            &[MARKER_FAILURE.to_owned(), other_failure.to_owned()],
            &[],
        ));
    }
}

#[test]
fn non_parity_event_failures_are_not_retryable() {
    for event_failure in [
        "model: completion emitted no nonempty text delta",
        "model: emitted an empty assistant delta",
        "model: emitted an empty reasoning delta",
        "model: direct provider probe unexpectedly restarted",
    ] {
        assert!(!should_retry_marker_miss(
            &[MARKER_FAILURE.to_owned()],
            &[event_failure.to_owned()],
        ));
    }
    assert!(!should_retry_marker_miss(
        &[MARKER_FAILURE.to_owned()],
        &[
            "model: assistant events did not have terminal parity".to_owned(),
            "model: emitted an empty reasoning delta".to_owned(),
        ],
    ));
}

#[tokio::test]
async fn runner_retries_one_marker_miss_and_uses_the_second_reply() {
    let provider = LiveProvider::MiniMax;
    let spec = first_spec(provider);
    let calls = Arc::new(AtomicUsize::new(0));
    let model = scripted_model(
        vec![
            valid_response(&spec, "not the requested reply"),
            valid_response(&spec, "LIVE_MODEL_API_OK"),
        ],
        calls.clone(),
    );

    let failures = run_probe_with_marker_retry(
        provider,
        &spec,
        spec.default_effort,
        model,
        ModelCompletionMode::Synchronous,
        provider.synchronous_event_expectation(),
    )
    .await;

    assert!(failures.is_empty());
    assert_eq!(calls.load(Ordering::SeqCst), 2);
}

#[tokio::test]
async fn runner_reports_the_second_marker_miss_after_one_retry() {
    let provider = LiveProvider::MiniMax;
    let spec = first_spec(provider);
    let calls = Arc::new(AtomicUsize::new(0));
    let model = scripted_model(
        vec![
            valid_response(&spec, "first miss"),
            valid_response(&spec, "second miss"),
        ],
        calls.clone(),
    );

    let failures = run_probe_with_marker_retry(
        provider,
        &spec,
        spec.default_effort,
        model,
        ModelCompletionMode::Synchronous,
        provider.synchronous_event_expectation(),
    )
    .await;

    assert_eq!(
        failures,
        vec![format!(
            "{}: assistant response did not contain the probe marker",
            spec.text_id()
        )]
    );
    assert_eq!(calls.load(Ordering::SeqCst), 2);
}

#[test]
fn probe_controls_are_provider_neutral() {
    let controls = probe_controls(ModelCompletionMode::PreferDeferred, Some(Effort::Medium));

    assert_eq!(controls.generation.tool_choice, Some(ModelToolChoice::None));
    assert_eq!(controls.execution.total_timeout, Some(MODEL_TIMEOUT));
    assert_eq!(
        controls.execution.completion_mode,
        ModelCompletionMode::PreferDeferred
    );
}

#[tokio::test]
async fn generic_runtime_executes_a_dynamic_model() {
    let response = complete_through_runtime(
        Arc::new(MockModel::new("live-probe")),
        ModelCompletionMode::Synchronous,
        Some(Effort::Disabled),
    )
    .await
    .expect("generic runtime should complete through the model trait");

    assert_eq!(response.response.provider, "mock");
    assert_eq!(response.response.model_id, "live-probe");
    assert!(!response.events.is_empty());
}

struct ScriptedModel {
    responses: Mutex<VecDeque<ModelResponse>>,
    calls: Arc<AtomicUsize>,
}

impl ScriptedModel {
    fn next_response(&self) -> ModelResponse {
        self.calls.fetch_add(1, Ordering::SeqCst);
        self.responses
            .lock()
            .expect("response lock should not be poisoned")
            .pop_front()
            .expect("unexpected scripted model call")
    }
}

#[async_trait]
impl Model for ScriptedModel {
    async fn stream(&self, _request: &ModelRequest) -> ModelResult<DynModelEventStream> {
        Ok(model_response_stream(self.next_response()))
    }
}

fn scripted_model(responses: Vec<ModelResponse>, calls: Arc<AtomicUsize>) -> DynModel {
    Arc::new(ScriptedModel {
        responses: Mutex::new(VecDeque::from(responses)),
        calls,
    })
}

fn first_spec(provider: LiveProvider) -> ModelSpec {
    provider
        .chat_catalog()
        .into_iter()
        .next()
        .expect("MiniMax chat catalog should not be empty")
}

fn valid_response(spec: &ModelSpec, assistant_message: &str) -> ModelResponse {
    ModelResponse {
        provider: spec.provider.as_str().to_owned(),
        model_id: spec.provider_model_id.to_string(),
        catalog_model_id: Some(spec.text_id().to_owned()),
        thinking_level: spec.default_effort,
        parts: vec![ai_interface::ConversationPart::text(
            assistant_message.to_owned(),
        )],
        finish_reason: FinishReason::Stop,
        structured_output: None,

        attempts: Vec::new(),
        usage: ModelUsage {
            input_tokens: 1,
            output_tokens: 1,
            total_tokens: 2,
            ..Default::default()
        },
    }
}
