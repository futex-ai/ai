use std::sync::Arc;

use ai_interface::{
    FinishReason, Model, ModelError, ModelMock, ModelResponse, ModelUsage, ToolCall,
};
use serde_json::json;
use unimock::{MockFn, Unimock, matching};

use crate::{StepOutcome, Turn};

use super::super::support::{TypedEchoTool, runtime, user_message};

#[tokio::test]
async fn model_errors_abort_the_turn() {
    #[derive(Debug, thiserror::Error)]
    #[error("[ai_tool_calling/tests] model failed")]
    struct FixtureModelError;

    let model: Arc<dyn Model> = Arc::new(Unimock::new(
        ModelMock::complete
            .next_call(matching!(_))
            .returns(Err(ModelError::internal(FixtureModelError))),
    ));
    let runtime = runtime(model, Vec::new()).expect("runtime should build");

    let mut turn = runtime.send(user_message("start"), Some(4));
    let error = turn.step().await.expect_err("model error should abort");
    assert!(matches!(error, crate::Error::Model(_)));
}

#[tokio::test]
async fn stop_finish_reason_completes_the_turn() {
    let model = single_response_model(model_response(FinishReason::Stop, Vec::new()));
    let runtime = runtime(model, Vec::new()).expect("runtime should build");

    let mut turn = runtime.send(user_message("start"), Some(4));
    let outcome = turn.step().await.expect("step should complete");

    assert!(matches!(
        outcome,
        StepOutcome::Completed {
            assistant_message,
            steps_taken: 1
        } if assistant_message == "done"
    ));
}

#[tokio::test]
async fn tool_calls_finish_reason_dispatches_tools() {
    let model = single_response_model(model_response(
        FinishReason::ToolCalls,
        vec![echo_tool_call()],
    ));
    let tool = TypedEchoTool::succeeding();
    let runtime = runtime(model, vec![tool.tool()]).expect("runtime should build");

    let mut turn = runtime.send(user_message("start"), Some(4));
    let outcome = turn.step().await.expect("step should dispatch tool");

    assert!(matches!(
        outcome,
        StepOutcome::Stepped { steps_taken: 1, .. }
    ));
}

#[tokio::test]
async fn tool_calls_finish_reason_without_calls_is_a_model_error() {
    let model = single_response_model(model_response(FinishReason::ToolCalls, Vec::new()));
    let runtime = runtime(model, Vec::new()).expect("runtime should build");

    let mut turn = runtime.send(user_message("start"), Some(4));
    let error = turn.step().await.expect_err("step should reject response");

    assert!(matches!(
        error,
        crate::Error::Model(ModelError::Provider { .. })
    ));
}

#[tokio::test]
async fn non_tool_finish_reason_with_calls_is_a_model_error() {
    let model = single_response_model(model_response(FinishReason::Stop, vec![echo_tool_call()]));
    let runtime = runtime(model, Vec::new()).expect("runtime should build");

    let mut turn = runtime.send(user_message("start"), Some(4));
    let error = turn.step().await.expect_err("step should reject response");

    assert!(matches!(
        error,
        crate::Error::Model(ModelError::Provider { .. })
    ));
}

#[tokio::test]
async fn truncated_finish_reason_completes_the_turn() {
    let model = single_response_model(model_response(FinishReason::Truncated, Vec::new()));
    let runtime = runtime(model, Vec::new()).expect("runtime should build");

    let mut turn = runtime.send(user_message("start"), Some(4));
    let outcome = turn.step().await.expect("step should complete");

    assert!(matches!(
        outcome,
        StepOutcome::Completed { steps_taken: 1, .. }
    ));
}

#[tokio::test]
async fn filtered_finish_reason_is_a_model_error() {
    let model = single_response_model(model_response(FinishReason::Filtered, Vec::new()));
    let runtime = runtime(model, Vec::new()).expect("runtime should build");

    let mut turn = runtime.send(user_message("start"), Some(4));
    let error = turn.step().await.expect_err("step should reject response");

    assert!(matches!(
        error,
        crate::Error::Model(ModelError::Provider { .. })
    ));
}

#[tokio::test]
async fn other_finish_reason_is_a_model_error_with_raw_reason() {
    let model = single_response_model(model_response(
        FinishReason::Other("custom".to_owned()),
        Vec::new(),
    ));
    let runtime = runtime(model, Vec::new()).expect("runtime should build");

    let mut turn = runtime.send(user_message("start"), Some(4));
    let error = turn.step().await.expect_err("step should reject response");

    assert!(matches!(
        error,
        crate::Error::Model(ModelError::Provider { message, .. }) if message.contains("custom")
    ));
}

fn single_response_model(response: ModelResponse) -> Arc<dyn Model> {
    Arc::new(Unimock::new(
        ModelMock::complete
            .next_call(matching!(_))
            .returns(Ok(response)),
    ))
}

fn model_response(finish_reason: FinishReason, tool_calls: Vec<ToolCall>) -> ModelResponse {
    ModelResponse {
        provider: "mock".to_owned(),
        model_id: "mock-model".to_owned(),
        catalog_model_id: None,
        thinking_level: None,
        assistant_message: "done".to_owned(),
        tool_calls,
        finish_reason,
        structured_output: None,
        provider_context: Vec::new(),
        usage: ModelUsage::default(),
    }
}

fn echo_tool_call() -> ToolCall {
    ToolCall {
        id: "call-1".to_owned(),
        name: "echo".to_owned(),
        input: json!({ "message": "hello" }),
        operation_id: None,
    }
}
