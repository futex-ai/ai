//! Catalog iteration and normalized live-response assertions.

use std::{env, sync::Arc, time::Duration};

use ai_interface::{
    ConversationMessage, DynModel, FinishReason, MockModel, ModelCallControls, ModelCompletionMode,
    ModelExecutionControls, ModelGenerationControls, ModelResponse, ModelToolChoice, NoopLogger,
};
use ai_models_core::{KnownModelSpec, RetryingModel};
use ai_tool_calling::{
    InMemoryToolOutputStore, ModelResponseCheckpoint, NoopTurnCheckpoint, StepOutcome,
    ToolCallingRuntime, ToolOutputPolicy,
};
use json_http::{JsonHttpClient, ReqwestJsonHttpClient};

use super::provider_tests::LiveProvider;

const API_KEY_ENV: &str = "LIVE_MODEL_API_KEY";
const EXPECTED_TEXT: &str = "LIVE_MODEL_API_OK";
const MODEL_TIMEOUT: Duration = Duration::from_secs(10 * 60);

pub(super) async fn run_catalog(provider: LiveProvider) {
    let api_key = env::var(API_KEY_ENV)
        .unwrap_or_else(|_| panic!("{API_KEY_ENV} must contain the provider API credential"));
    assert!(
        !api_key.trim().is_empty(),
        "{API_KEY_ENV} must not be empty"
    );

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
        match complete_through_runtime(model).await {
            Ok(response) => validate_response(provider, spec, &response, &mut failures),
            Err(error) => failures.push(format!("{}: request failed: {error}", spec.id)),
        }
    }

    assert!(
        failures.is_empty(),
        "{} live catalog failure(s):\n{}",
        failures.len(),
        failures.join("\n")
    );
}

async fn complete_through_runtime(model: DynModel) -> Result<ModelResponse, String> {
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
        .with_controls(probe_controls());
    let mut turn_checkpoint = NoopTurnCheckpoint;
    let mut response_checkpoint = ResponseCapture::default();
    let outcome = turn
        .step_with_checkpoints(&mut turn_checkpoint, &mut response_checkpoint)
        .await
        .map_err(|error| format!("generic runtime execution failed: {error}"))?;
    if !matches!(outcome, StepOutcome::Completed { steps_taken: 1, .. }) {
        return Err(format!("generic runtime returned {outcome:?}"));
    }
    response_checkpoint
        .response
        .ok_or_else(|| "generic runtime did not expose the model response".to_owned())
}

fn probe_controls() -> ModelCallControls {
    ModelCallControls {
        generation: ModelGenerationControls {
            tool_choice: Some(ModelToolChoice::None),
            ..Default::default()
        },
        execution: ModelExecutionControls {
            total_timeout: Some(MODEL_TIMEOUT),
            completion_mode: ModelCompletionMode::PreferDeferred,
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

#[test]
fn probe_controls_are_provider_neutral() {
    let controls = probe_controls();

    assert_eq!(controls.generation.tool_choice, Some(ModelToolChoice::None));
    assert_eq!(controls.execution.total_timeout, Some(MODEL_TIMEOUT));
    assert_eq!(
        controls.execution.completion_mode,
        ModelCompletionMode::PreferDeferred
    );
}

#[tokio::test]
async fn generic_runtime_executes_a_dynamic_model() {
    let response = complete_through_runtime(Arc::new(MockModel::new("live-probe")))
        .await
        .expect("generic runtime should complete through the model trait");

    assert_eq!(response.provider, "mock");
    assert_eq!(response.model_id, "live-probe");
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
        failures.push(format!(
            "{}: assistant response did not contain the probe marker",
            spec.id
        ));
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
