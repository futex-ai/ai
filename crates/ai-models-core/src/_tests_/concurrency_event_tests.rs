//! Event pass-through tests for the concurrency wrapper.

use std::sync::Arc;

use ai_interface::test_support::model_response_stream;
use ai_interface::{FinishReason, Model, ModelMock, ModelRequest, ModelResponse, ModelUsage};
use unimock::{MockFn, Unimock, matching};

use crate::ConcurrencyLimitedModel;

#[tokio::test]
async fn concurrency_wrapper_uses_the_event_observing_entrypoint() {
    let inner: Arc<dyn Model> = Arc::new(Unimock::new(
        ModelMock::stream
            .next_call(matching!(_))
            .answers(&|_, _request: &ModelRequest| Ok(model_response_stream(success_response()))),
    ));
    let model = ConcurrencyLimitedModel::new(inner, "mock", 1);

    model
        .complete(&empty_request())
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
        parts: vec![ai_interface::ConversationPart::text("ok".to_owned())],
        finish_reason: FinishReason::Stop,
        structured_output: None,

        usage: ModelUsage::default(),
        attempts: Vec::new(),
    }
}
