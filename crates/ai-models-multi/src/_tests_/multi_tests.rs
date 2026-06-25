//! Tests for ordered multi-model fallback behavior.

use std::sync::{
    Arc, Mutex,
    atomic::{AtomicUsize, Ordering},
};

use ai_interface::{
    FinishReason, Model, ModelError, ModelMock, ModelRequest, ModelResponse, ModelUsage,
};
use unimock::{MockFn, Unimock, matching};

use crate::MultiModel;

#[tokio::test]
async fn falls_back_on_rate_limits() {
    let first_calls = Arc::new(AtomicUsize::new(0));
    let second_calls = Arc::new(AtomicUsize::new(0));
    let model = MultiModel::new(vec![
        scripted_model(
            first_calls.clone(),
            Err(ModelError::rate_limited("openai", "gpt-5.5", "slow down")),
        ),
        scripted_model(
            second_calls.clone(),
            Ok(success_response("anthropic", "claude-sonnet-4-6")),
        ),
    ]);

    let response = model
        .complete(&empty_request())
        .await
        .expect("fallback should succeed");

    assert_eq!(response.provider, "anthropic");
    assert_eq!(first_calls.load(Ordering::SeqCst), 1);
    assert_eq!(second_calls.load(Ordering::SeqCst), 1);
}

#[tokio::test]
async fn falls_back_on_transient_provider_failures() {
    let first_calls = Arc::new(AtomicUsize::new(0));
    let second_calls = Arc::new(AtomicUsize::new(0));
    let model = MultiModel::new(vec![
        scripted_model(
            first_calls.clone(),
            Err(ModelError::transient_provider(
                "openai",
                "gpt-5.5",
                "temporary outage",
            )),
        ),
        scripted_model(
            second_calls.clone(),
            Ok(success_response("google", "gemini-2.5-pro")),
        ),
    ]);

    let response = model
        .complete(&empty_request())
        .await
        .expect("fallback should succeed");

    assert_eq!(response.provider, "google");
    assert_eq!(first_calls.load(Ordering::SeqCst), 1);
    assert_eq!(second_calls.load(Ordering::SeqCst), 1);
}

#[tokio::test]
async fn falls_back_on_provider_failures() {
    let first_calls = Arc::new(AtomicUsize::new(0));
    let second_calls = Arc::new(AtomicUsize::new(0));
    let model = MultiModel::new(vec![
        scripted_model(
            first_calls.clone(),
            Err(ModelError::provider("openai", "gpt-5.5", "bad request")),
        ),
        scripted_model(
            second_calls.clone(),
            Ok(success_response("anthropic", "claude-sonnet-4-6")),
        ),
    ]);

    let response = model
        .complete(&empty_request())
        .await
        .expect("fallback should succeed");

    assert_eq!(response.provider, "anthropic");
    assert_eq!(first_calls.load(Ordering::SeqCst), 1);
    assert_eq!(second_calls.load(Ordering::SeqCst), 1);
}

#[tokio::test]
async fn falls_back_on_context_limit_failures() {
    let first_calls = Arc::new(AtomicUsize::new(0));
    let second_calls = Arc::new(AtomicUsize::new(0));
    let model = MultiModel::new(vec![
        scripted_model(
            first_calls.clone(),
            Err(ModelError::context_limit_exceeded(
                "openai",
                "gpt-5.5",
                "too large",
            )),
        ),
        scripted_model(
            second_calls.clone(),
            Ok(success_response("google", "gemini-2.5-pro")),
        ),
    ]);

    let response = model
        .complete(&empty_request())
        .await
        .expect("fallback should succeed");

    assert_eq!(response.provider, "google");
    assert_eq!(first_calls.load(Ordering::SeqCst), 1);
    assert_eq!(second_calls.load(Ordering::SeqCst), 1);
}

#[tokio::test]
async fn falls_back_on_internal_failures() {
    let first_calls = Arc::new(AtomicUsize::new(0));
    let second_calls = Arc::new(AtomicUsize::new(0));
    let model = MultiModel::new(vec![
        scripted_model(
            first_calls.clone(),
            Err(ModelError::internal(std::io::Error::other(
                "adapter failed",
            ))),
        ),
        scripted_model(
            second_calls.clone(),
            Ok(success_response("xai", "grok-4.20-reasoning")),
        ),
    ]);

    let response = model
        .complete(&empty_request())
        .await
        .expect("fallback should succeed");

    assert_eq!(response.provider, "xai");
    assert_eq!(first_calls.load(Ordering::SeqCst), 1);
    assert_eq!(second_calls.load(Ordering::SeqCst), 1);
}

fn scripted_model(
    calls: Arc<AtomicUsize>,
    result: std::result::Result<ModelResponse, ModelError>,
) -> Arc<dyn Model> {
    let result = Arc::new(Mutex::new(Some(result)));
    Arc::new(Unimock::new(ModelMock::complete.stub(move |each| {
        let calls = calls.clone();
        let result = result.clone();
        each.call(matching!(_))
            .answers_arc(Arc::new(move |_, _request: &ModelRequest| {
                calls.fetch_add(1, Ordering::SeqCst);
                result
                    .lock()
                    .expect("result lock should not be poisoned")
                    .take()
                    .expect("unexpected model call")
            }));
    })))
}

fn empty_request() -> ModelRequest {
    ModelRequest {
        system_prompt: "system".to_owned(),
        messages: Vec::new(),
        tools: Vec::new(),
        response_schema: None,
    }
}

fn success_response(provider: &str, model_id: &str) -> ModelResponse {
    ModelResponse {
        provider: provider.to_owned(),
        model_id: model_id.to_owned(),
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
