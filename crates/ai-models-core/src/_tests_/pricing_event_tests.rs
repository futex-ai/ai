//! Event pass-through tests for the usage-pricing wrapper.

use std::sync::Arc;

use ai_interface::{
    FinishReason, Model, ModelCompletionEventSink, ModelMock, ModelRequest, ModelResponse,
    ModelUsage,
};
use unimock::{MockFn, Unimock, matching};

use crate::{ModelPricing, UsagePricingModel};

#[tokio::test]
async fn pricing_wrapper_prices_the_event_observing_response() {
    let inner: Arc<dyn Model> = Arc::new(Unimock::new(
        ModelMock::complete_with_events
            .next_call(matching!(_, _))
            .answers(
                &|_, _request: &ModelRequest, _sink: &dyn ModelCompletionEventSink| {
                    let mut response = success_response();
                    response.usage.input_tokens = 1_000_000;
                    response.usage.total_tokens = 1_000_000;
                    Ok(response)
                },
            ),
    ));
    let model = UsagePricingModel::new(
        inner,
        ModelPricing {
            input_token_usd_micros_per_million: Some(25),
            ..ModelPricing::default()
        },
    );
    let sink = Unimock::new(());

    let response = model
        .complete_with_events(&empty_request(), &sink)
        .await
        .expect("event-observing call should be priced");

    assert_eq!(response.usage.estimated_cost_microusd, 25);
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
