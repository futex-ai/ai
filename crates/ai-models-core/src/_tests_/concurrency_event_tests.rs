//! Event pass-through tests for the concurrency wrapper.

use std::sync::Arc;

use ai_interface::{
    FinishReason, Model, ModelCompletionEventSink, ModelMock, ModelRequest, ModelResponse,
    ModelUsage,
};
use unimock::{MockFn, Unimock, matching};

use crate::ConcurrencyLimitedModel;

#[tokio::test]
async fn concurrency_wrapper_uses_the_event_observing_entrypoint() {
    let inner: Arc<dyn Model> = Arc::new(Unimock::new(
        ModelMock::complete_with_events
            .next_call(matching!(_, _))
            .answers(
                &|_, _request: &ModelRequest, _sink: &dyn ModelCompletionEventSink| {
                    Ok(success_response())
                },
            ),
    ));
    let model = ConcurrencyLimitedModel::new(inner, "mock", 1);
    let sink = Unimock::new(());

    model
        .complete_with_events(&empty_request(), &sink)
        .await
        .expect("event-observing call should pass through");
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
