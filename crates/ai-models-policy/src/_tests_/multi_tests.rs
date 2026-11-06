//! Tests for ordered fallback behavior preserved from ai-models-multi.

use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};

use ai_interface::{Model, ModelError, ModelGenerationControls, ModelToolChoice, SafeErrorSource};

use crate::{AnyError, FallbackModel};

use super::multi_fixtures::{
    control_asserting_model, empty_request, scripted_model, success_response,
};

#[tokio::test]
async fn falls_back_on_rate_limits() {
    let first_calls = Arc::new(AtomicUsize::new(0));
    let second_calls = Arc::new(AtomicUsize::new(0));
    let model = FallbackModel::new(
        vec![
            scripted_model(
                first_calls.clone(),
                Err(ModelError::rate_limited("openai", "gpt-5.5", "slow down")),
            ),
            scripted_model(
                second_calls.clone(),
                Ok(success_response("anthropic", "claude-sonnet-4-6")),
            ),
        ],
        std::sync::Arc::new(AnyError),
    );

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
    let model = FallbackModel::new(
        vec![
            scripted_model(
                first_calls.clone(),
                Err(ModelError::transient(
                    "openai",
                    "gpt-5.5",
                    "temporary outage",
                )),
            ),
            scripted_model(
                second_calls.clone(),
                Ok(success_response("google", "gemini-2.5-pro")),
            ),
        ],
        std::sync::Arc::new(AnyError),
    );

    let response = model
        .complete(&empty_request())
        .await
        .expect("fallback should succeed");

    assert_eq!(response.provider, "google");
    assert_eq!(first_calls.load(Ordering::SeqCst), 1);
    assert_eq!(second_calls.load(Ordering::SeqCst), 1);
}

#[tokio::test]
async fn falls_back_after_an_interrupted_generation() {
    let first_calls = Arc::new(AtomicUsize::new(0));
    let second_calls = Arc::new(AtomicUsize::new(0));
    let model = FallbackModel::new(
        vec![
            scripted_model(
                first_calls.clone(),
                Err(ModelError::interrupted(
                    "openai",
                    "gpt-5.5",
                    "stream ended after output",
                )),
            ),
            scripted_model(
                second_calls.clone(),
                Ok(success_response("google", "gemini-2.5-pro")),
            ),
        ],
        std::sync::Arc::new(AnyError),
    );

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
    let model = FallbackModel::new(
        vec![
            scripted_model(
                first_calls.clone(),
                Err(ModelError::invalid_request(
                    "openai",
                    "gpt-5.5",
                    "bad request",
                )),
            ),
            scripted_model(
                second_calls.clone(),
                Ok(success_response("anthropic", "claude-sonnet-4-6")),
            ),
        ],
        std::sync::Arc::new(AnyError),
    );

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
    let model = FallbackModel::new(
        vec![
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
        ],
        std::sync::Arc::new(AnyError),
    );

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
    let model = FallbackModel::new(
        vec![
            scripted_model(
                first_calls.clone(),
                Err(ModelError::internal(
                    "openai",
                    "gpt-5.5",
                    SafeErrorSource::redacted(std::io::Error::other("adapter failed")),
                )),
            ),
            scripted_model(
                second_calls.clone(),
                Ok(success_response("xai", "grok-4.20-reasoning")),
            ),
        ],
        std::sync::Arc::new(AnyError),
    );

    let response = model
        .complete(&empty_request())
        .await
        .expect("fallback should succeed");

    assert_eq!(response.provider, "xai");
    assert_eq!(first_calls.load(Ordering::SeqCst), 1);
    assert_eq!(second_calls.load(Ordering::SeqCst), 1);
}

#[tokio::test]
async fn forwards_call_controls_to_each_fallback_candidate() {
    let first_calls = Arc::new(AtomicUsize::new(0));
    let second_calls = Arc::new(AtomicUsize::new(0));
    let model = FallbackModel::new(
        vec![
            control_asserting_model(
                first_calls.clone(),
                Err(ModelError::transient("one", "one", "retry")),
            ),
            control_asserting_model(second_calls.clone(), Ok(success_response("two", "two"))),
        ],
        std::sync::Arc::new(AnyError),
    );
    let mut request = empty_request();
    request.controls.generation = ModelGenerationControls {
        tool_choice: Some(ModelToolChoice::RequiredOrAuto),
        ..Default::default()
    };

    model
        .complete(&request)
        .await
        .expect("fallback should succeed");

    assert_eq!(first_calls.load(Ordering::SeqCst), 1);
    assert_eq!(second_calls.load(Ordering::SeqCst), 1);
}
