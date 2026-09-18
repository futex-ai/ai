//! Catalog iteration and normalized live-response assertions.

use std::{env, sync::Arc, time::Duration};

use ai_interface::{
    ConversationMessage, DynModel, FinishReason, ModelCallControls, ModelCompletionEvent,
    ModelCompletionMode, ModelExecutionControls, ModelGenerationControls, ModelResponse,
    ModelToolChoice, NoopLogger,
};
use ai_models_core::{KnownModelSpec, RetryingModel};
use ai_tool_calling::{
    InMemoryToolOutputStore, ModelResponseCheckpoint, NoopTurnCheckpoint, StepOutcome,
    ToolCallingRuntime, ToolOutputPolicy,
};
use json_http::{JsonHttpClient, ReqwestJsonHttpClient};

use super::{
    event_tests::{completion_event_failures, observing_model},
    provider_tests::{CompletionEventExpectation, LiveProvider},
};

const API_KEY_ENV: &str = "LIVE_MODEL_API_KEY";
const EXPECTED_TEXT: &str = "LIVE_MODEL_API_OK";
const MARKER_MISS_FAILURE: &str = "assistant response did not contain the probe marker";
const EVENT_PARITY_FAILURE: &str = "assistant events did not have terminal parity";
pub(super) const MODEL_TIMEOUT: Duration = Duration::from_secs(10 * 60);

pub(super) async fn run_live_provider(provider: LiveProvider) {
    run_synchronous_stream_probe(provider).await;
    run_catalog(provider).await;
}

async fn run_catalog(provider: LiveProvider) {
    let api_key = live_api_key();

    let client: Arc<dyn JsonHttpClient> = Arc::new(ReqwestJsonHttpClient::new());
    let auth = provider.auth(api_key);
    let catalog = provider.chat_catalog();
    assert!(
        !catalog.is_empty(),
        "{provider:?} catalog must not be empty"
    );

    let mut failures = Vec::new();
    for spec in &catalog {
        println!("checking {}/{}", provider.kind(), spec.id);
        let model: DynModel = Arc::new(RetryingModel::with_standard_transient_retry(
            provider.build(client.clone(), auth.clone(), spec),
        ));
        failures.extend(
            run_probe_with_marker_retry(
                provider,
                spec,
                model,
                ModelCompletionMode::PreferDeferred,
                provider.preferred_mode_event_expectation(),
            )
            .await,
        );
    }

    assert!(
        failures.is_empty(),
        "{} live catalog failure(s):\n{}",
        failures.len(),
        failures.join("\n")
    );
}

async fn run_synchronous_stream_probe(provider: LiveProvider) {
    let api_key = live_api_key();
    let spec = provider
        .chat_catalog()
        .into_iter()
        .next()
        .expect("live provider chat catalog must not be empty");
    let client: Arc<dyn JsonHttpClient> = Arc::new(ReqwestJsonHttpClient::new());
    let model: DynModel = Arc::new(RetryingModel::with_standard_transient_retry(
        provider.build(client, provider.auth(api_key), &spec),
    ));
    let failures = run_probe_with_marker_retry(
        provider,
        &spec,
        model,
        ModelCompletionMode::Synchronous,
        provider.synchronous_event_expectation(),
    )
    .await;
    assert!(
        failures.is_empty(),
        "synchronous stream probe failure(s):\n{}",
        failures.join("\n")
    );
}

pub(super) async fn run_probe_with_marker_retry(
    provider: LiveProvider,
    spec: &KnownModelSpec,
    model: DynModel,
    completion_mode: ModelCompletionMode,
    event_expectation: CompletionEventExpectation,
) -> Vec<String> {
    let mut retry_available = true;
    loop {
        let completion = match complete_through_runtime(model.clone(), completion_mode).await {
            Ok(completion) => completion,
            Err(error) => return vec![format!("{}: request failed: {error}", spec.id)],
        };
        let mut response_failures = Vec::new();
        validate_response(provider, spec, &completion.response, &mut response_failures);
        let event_failures = completion_event_failures(
            spec.id,
            event_expectation,
            &completion.response.assistant_message,
            &completion.events,
        );
        if retry_available && should_retry_marker_miss(&response_failures, &event_failures) {
            println!(
                "retrying {}/{} after a probe-marker miss",
                provider.kind(),
                spec.id
            );
            retry_available = false;
            continue;
        }
        response_failures.extend(event_failures);
        return response_failures;
    }
}

pub(super) fn should_retry_marker_miss(
    response_failures: &[String],
    event_failures: &[String],
) -> bool {
    response_failures.len() == 1
        && has_failure_message(&response_failures[0], MARKER_MISS_FAILURE)
        && match event_failures {
            [] => true,
            [failure] => has_failure_message(failure, EVENT_PARITY_FAILURE),
            _ => false,
        }
}

fn has_failure_message(failure: &str, expected: &str) -> bool {
    failure
        .rsplit_once(": ")
        .is_some_and(|(_, message)| message == expected)
}

pub(super) async fn complete_through_runtime(
    model: DynModel,
    completion_mode: ModelCompletionMode,
) -> Result<ObservedCompletion, String> {
    let (model, event_log) = observing_model(model);
    let runtime = ToolCallingRuntime::new(
        format!(
            "You are a CI connectivity probe. Reply with exactly {EXPECTED_TEXT} and no other text."
        ),
        model,
        Arc::new(NoopLogger),
        Vec::new(),
        Arc::new(InMemoryToolOutputStore::new()),
        ToolOutputPolicy::default(),
    )
    .map_err(|error| format!("generic runtime construction failed: {error}"))?;
    let mut turn = runtime
        .send(
            ConversationMessage::user(format!("Reply with {EXPECTED_TEXT}.")),
            Some(1),
        )
        .with_controls(probe_controls(completion_mode));
    let mut turn_checkpoint = NoopTurnCheckpoint;
    let mut response_checkpoint = ResponseCapture::default();
    let outcome = turn
        .step_with_checkpoints(&mut turn_checkpoint, &mut response_checkpoint)
        .await
        .map_err(|error| format!("generic runtime execution failed: {error}"))?;
    if !matches!(outcome, StepOutcome::Completed { steps_taken: 1, .. }) {
        return Err(format!("generic runtime returned {outcome:?}"));
    }
    let response = response_checkpoint
        .response
        .ok_or_else(|| "generic runtime did not expose the model response".to_owned())?;
    Ok(ObservedCompletion {
        response,
        events: event_log.recorded(),
    })
}

pub(super) struct ObservedCompletion {
    pub(super) response: ModelResponse,
    pub(super) events: Vec<ModelCompletionEvent>,
}

pub(super) fn probe_controls(completion_mode: ModelCompletionMode) -> ModelCallControls {
    ModelCallControls {
        generation: ModelGenerationControls {
            tool_choice: Some(ModelToolChoice::None),
            ..Default::default()
        },
        execution: ModelExecutionControls {
            total_timeout: Some(MODEL_TIMEOUT),
            completion_mode,
        },
    }
}

#[derive(Default)]
struct ResponseCapture {
    response: Option<ModelResponse>,
}

impl ModelResponseCheckpoint for ResponseCapture {
    fn checkpoint_response(&mut self, response: &mut ModelResponse) -> ai_tool_calling::Result<()> {
        self.response = Some(response.clone());
        Ok(())
    }
}

fn live_api_key() -> String {
    let api_key = env::var(API_KEY_ENV)
        .unwrap_or_else(|_| panic!("{API_KEY_ENV} must contain the provider API credential"));
    assert!(
        !api_key.trim().is_empty(),
        "{API_KEY_ENV} must not be empty"
    );
    api_key
}

fn validate_response(
    provider: LiveProvider,
    spec: &KnownModelSpec,
    response: &ModelResponse,
    failures: &mut Vec<String>,
) {
    let expected_provider = provider.kind().as_str();
    if response.provider != expected_provider {
        failures.push(format!(
            "{}: provider was `{}`, expected `{expected_provider}`",
            spec.id, response.provider
        ));
    }
    if response.model_id != spec.provider_model_id {
        failures.push(format!(
            "{}: provider model was `{}`, expected `{}`",
            spec.id, response.model_id, spec.provider_model_id
        ));
    }
    if response.catalog_model_id.as_deref() != Some(spec.id) {
        failures.push(format!(
            "{}: catalog model was {:?}",
            spec.id, response.catalog_model_id
        ));
    }
    if response.thinking_level.as_deref() != Some(spec.thinking_level.as_str()) {
        failures.push(format!(
            "{}: thinking level was {:?}, expected `{}`",
            spec.id,
            response.thinking_level,
            spec.thinking_level.as_str()
        ));
    }
    if !matches!(response.finish_reason, FinishReason::Stop) {
        failures.push(format!(
            "{}: finish reason was {:?}",
            spec.id, response.finish_reason
        ));
    }
    if !response.assistant_message.contains(EXPECTED_TEXT) {
        failures.push(format!("{}: {MARKER_MISS_FAILURE}", spec.id));
    }
    if !response.tool_calls.is_empty() {
        failures.push(format!(
            "{}: response unexpectedly requested tools",
            spec.id
        ));
    }
    if response.usage.total_tokens == 0 {
        failures.push(format!("{}: provider reported zero total tokens", spec.id));
    }
}
