//! Catalog iteration and normalized live-response assertions.

use std::env;
use std::sync::Arc;

use ai_interface::{ConversationMessage, FinishReason, Model, ModelRequest, ModelResponse};
use ai_models_core::{KnownModelSpec, RetryingModel};
use json_http::{JsonHttpClient, ReqwestJsonHttpClient};

use super::provider_tests::LiveProvider;

const API_KEY_ENV: &str = "LIVE_MODEL_API_KEY";
const EXPECTED_TEXT: &str = "LIVE_MODEL_API_OK";

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
        let model = RetryingModel::with_standard_transient_retry(provider.build(
            client.clone(),
            auth.clone(),
            spec,
        ));
        match model.complete(&probe_request()).await {
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

fn probe_request() -> ModelRequest {
    ModelRequest {
        system_prompt: format!(
            "You are a CI connectivity probe. Reply with exactly {EXPECTED_TEXT} and no other text."
        ),
        messages: vec![ConversationMessage::user(format!(
            "Reply with {EXPECTED_TEXT}."
        ))],
        tools: Vec::new(),
        response_schema: None,
    }
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
