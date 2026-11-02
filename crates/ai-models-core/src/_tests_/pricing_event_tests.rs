//! Event pass-through tests for the usage-pricing wrapper.

use std::sync::Arc;

use ai_interface::test_support::model_response_stream;
use ai_interface::{FinishReason, Model, ModelMock, ModelRequest, ModelResponse, ModelUsage};
use unimock::{MockFn, Unimock, matching};

use crate::{ModelPricing, UsagePricingModel};

#[tokio::test]
async fn pricing_wrapper_prices_the_event_observing_response() {
    let inner: Arc<dyn Model> = Arc::new(Unimock::new(
        ModelMock::stream
            .next_call(matching!(_))
            .answers(&|_, _request: &ModelRequest| {
                let mut response = success_response();
                response.usage.input_tokens = 1_000_000;
                response.usage.total_tokens = 1_000_000;
                Ok(model_response_stream(response))
            }),
    ));
    let model = UsagePricingModel::new(
        inner,
        ModelPricing {
            input_token_usd_micros_per_million: Some(25),
            ..ModelPricing::default()
        },
    );

    let response = model
        .complete(&empty_request())
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
        parts: vec![ai_interface::ConversationPart::text("ok".to_owned())],
        finish_reason: FinishReason::Stop,
        structured_output: None,

        usage: ModelUsage::default(),
        attempts: Vec::new(),
    }
}
