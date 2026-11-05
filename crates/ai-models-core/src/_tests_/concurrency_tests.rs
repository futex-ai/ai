//! Tests for concurrency limiting around model calls.

use std::{
    future::Future,
    pin::pin,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    task::{Context, Poll, Waker},
};

use ai_interface::test_support::model_response_stream;
use ai_interface::{
    FinishReason, Model, ModelGenerationControls, ModelMock, ModelRequest, ModelResponse,
    ModelToolChoice, ModelUsage,
};
use unimock::{MockFn, Unimock, matching};

use crate::ConcurrencyLimitedModel;

#[tokio::test]
async fn enforces_the_max_concurrent_limit() {
    let first_entered = Arc::new(AtomicBool::new(false));
    let second_entered = Arc::new(AtomicBool::new(false));
    let wrapped = ConcurrencyLimitedModel::new(
        blocking_model(first_entered.clone(), second_entered.clone()),
        "mock",
        1,
    );
    let request = controlled_request();
    let mut first = wrapped
        .stream(&request)
        .await
        .expect("first open should succeed");
    let mut second = pin!(wrapped.stream(&request));
    let mut context = Context::from_waker(Waker::noop());
    assert!(matches!(second.as_mut().poll(&mut context), Poll::Pending));

    assert!(
        !second_entered.load(Ordering::SeqCst),
        "second request should wait for the first permit"
    );
    while first
        .next()
        .await
        .expect("first call should succeed")
        .is_some()
    {}
    let mut second = second.await.expect("second open should succeed");
    while second
        .next()
        .await
        .expect("second call should succeed")
        .is_some()
    {}
    assert!(second_entered.load(Ordering::SeqCst));
}

fn blocking_model(
    first_entered: Arc<AtomicBool>,
    second_entered: Arc<AtomicBool>,
) -> Arc<dyn Model> {
    Arc::new(Unimock::new(
        ModelMock::stream.each_call(matching!(_)).answers_arc({
            let first_entered = first_entered.clone();
            let second_entered = second_entered.clone();
            Arc::new(move |_, request: &ModelRequest| {
                assert_eq!(
                    request.controls.generation.tool_choice,
                    Some(ModelToolChoice::RequiredOrAuto)
                );
                if first_entered
                    .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
                    .is_err()
                {
                    second_entered.store(true, Ordering::SeqCst);
                }
                Ok(model_response_stream(success_response()))
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
        controls: Default::default(),
    }
}

fn controlled_request() -> ModelRequest {
    let mut request = empty_request();
    request.controls.generation = ModelGenerationControls {
        tool_choice: Some(ModelToolChoice::RequiredOrAuto),
        ..Default::default()
    };
    request
}

fn success_response() -> ModelResponse {
    ModelResponse {
        provider: "mock".to_owned(),
        model_id: "mock".to_owned(),
        catalog_model_id: None,
        thinking_level: None,
        parts: vec![ai_interface::ConversationPart::text("ok".to_owned())],
        finish_reason: FinishReason::Stop,
        structured_output: None,

        usage: ModelUsage::default(),
        attempts: Vec::new(),
    }
}
