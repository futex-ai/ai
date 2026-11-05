//! Explicit scenario effort survives both calls of the tool loop.

use std::sync::{Arc, Mutex};

use ai_interface::{Effort, FinishReason, ModelMock, ModelRequest, ModelResponse, ToolCall};
use json_http::test_support::Redactor;
use serde_json::json;
use unimock::{MockFn, Unimock, matching};

use crate::{
    provider::ProviderModelFactory,
    runner::{ConformanceRunner, ScenarioRunner},
    scenario::Scenario,
};

#[tokio::test]
async fn execute_keeps_effort_on_the_initial_and_follow_up_requests() {
    let observed = Arc::new(Mutex::new(Vec::new()));
    let captured = observed.clone();
    let response = ModelResponse {
        provider: "provider-a".to_owned(),
        model_id: "model".to_owned(),
        catalog_model_id: Some("provider-a/model".to_owned()),
        thinking_level: Some(Effort::Low),
        assistant_message: String::new(),
        tool_calls: vec![ToolCall {
            id: "call_1".to_owned(),
            name: "lookup".to_owned(),
            input: json!({"id": 7}),
            operation_id: None,
        }],
        finish_reason: FinishReason::ToolCalls,
        structured_output: None,
        provider_context: Vec::new(),
        usage: Default::default(),
    };
    let model = Arc::new(Unimock::new(
        ModelMock::complete
            .each_call(matching!(_))
            .answers_arc(Arc::new(move |_, request: &ModelRequest| {
                captured
                    .lock()
                    .expect("capture lock")
                    .push(request.controls.generation.effort);
                Ok(response.clone())
            })),
    ));
    let runner = ScenarioRunner::new(Arc::new(ProviderModelFactory));
    let outcome = runner
        .execute(
            model,
            Scenario::ToolLoop,
            Some(Effort::Low),
            &Redactor::default(),
        )
        .await
        .expect("scenario executes");

    assert_eq!(
        *observed.lock().expect("capture lock"),
        vec![Some(Effort::Low), Some(Effort::Low)]
    );
    assert_eq!(outcome.steps.len(), 2);
}
