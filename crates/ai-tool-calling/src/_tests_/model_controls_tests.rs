//! Tool-calling model-control propagation tests.

use std::sync::Arc;

use ai_interface::test_support::model_response_stream;
use ai_interface::{
    ConversationMessage, FinishReason, Model, ModelCallControls, ModelCompletionMode,
    ModelExecutionControls, ModelGenerationControls, ModelMock, ModelRequest, ModelResponse,
    ModelToolChoice, ModelUsage,
};
use unimock::{MockFn, Unimock, matching};

use crate::Turn;
use crate::tests::support::response_parts;

use super::support::runtime;

#[tokio::test]
async fn active_turn_forwards_controls_to_every_model_step() {
    let controls = ModelCallControls {
        generation: ModelGenerationControls {
            effort: None,
            temperature: Some(0.25),
            top_p: Some(0.8),
            max_output_tokens: Some(321),
            stop_sequences: vec!["done".to_owned()],
            tool_choice: Some(ModelToolChoice::RequiredOrAuto),
        },
        execution: ModelExecutionControls {
            total_timeout: Some(std::time::Duration::from_secs(90)),
            completion_mode: ModelCompletionMode::PreferDeferred,
        },
    };
    let expected = controls.clone();
    let model: Arc<dyn Model> = Arc::new(Unimock::new(
        ModelMock::stream
            .next_call(matching!(_))
            .answers_arc(Arc::new(move |_, request: &ModelRequest| {
                {
                    assert_eq!(request.controls, expected);
                    Ok(response())
                }
                .map(model_response_stream)
            })),
    ));
    let runtime = runtime(model, Vec::new()).expect("runtime should build");

    let outcome = runtime
        .send(ConversationMessage::user("hello"), Some(1))
        .with_controls(controls)
        .run()
        .await
        .expect("turn should finish");

    assert!(matches!(outcome, crate::RunOutcome::Completed { .. }));
}

fn response() -> ModelResponse {
    ModelResponse {
        attempts: Vec::new(),
        provider: "test".to_owned(),
        model_id: "test-model".to_owned(),
        catalog_model_id: None,
        thinking_level: None,
        parts: response_parts("done".to_owned(), Vec::new(), Vec::new()),
        finish_reason: FinishReason::Stop,
        structured_output: None,
        usage: ModelUsage::default(),
    }
}
