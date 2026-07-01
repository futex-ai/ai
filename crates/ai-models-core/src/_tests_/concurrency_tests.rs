//! Tests for concurrency limiting around model calls.

use std::{
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    time::Duration,
};

use ai_interface::{FinishReason, Model, ModelMock, ModelRequest, ModelResponse, ModelUsage};
use unimock::{MockFn, Unimock, matching};

use crate::ConcurrencyLimitedModel;

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn enforces_the_max_concurrent_limit() {
    let first_entered = Arc::new(AtomicBool::new(false));
    let second_entered = Arc::new(AtomicBool::new(false));
    let wrapped = Arc::new(ConcurrencyLimitedModel::new(
        blocking_model(first_entered.clone(), second_entered.clone()),
        "mock",
        1,
    ));

    let first_model = wrapped.clone();
    let first = tokio::spawn(async move { first_model.complete(&empty_request()).await });
    while !first_entered.load(Ordering::SeqCst) {
        tokio::task::yield_now().await;
    }

    let second_model = wrapped.clone();
    let second = tokio::spawn(async move { second_model.complete(&empty_request()).await });
    tokio::time::sleep(Duration::from_millis(30)).await;

    assert!(
        !second_entered.load(Ordering::SeqCst),
        "second request should wait for the first permit"
    );

    first
        .await
        .expect("first task should join")
        .expect("first call should succeed");
    second
        .await
        .expect("second task should join")
        .expect("second call should succeed");
    assert!(second_entered.load(Ordering::SeqCst));
}

fn blocking_model(
    first_entered: Arc<AtomicBool>,
    second_entered: Arc<AtomicBool>,
) -> Arc<dyn Model> {
    Arc::new(Unimock::new(
        ModelMock::complete.each_call(matching!(_)).answers_arc({
            let first_entered = first_entered.clone();
            let second_entered = second_entered.clone();
            Arc::new(move |_, _request: &ModelRequest| {
                if first_entered
                    .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
                    .is_ok()
                {
                    std::thread::sleep(Duration::from_millis(150));
                } else {
                    second_entered.store(true, Ordering::SeqCst);
                }
                Ok(success_response())
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
        provider: "mock".to_owned(),
        model_id: "mock".to_owned(),
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
