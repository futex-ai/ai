use std::sync::Arc;

use ai_interface::{
    FinishReason, Model, ModelMock, ModelRequest, ModelResponse, ModelUsage, ToolCall, ToolError,
};
use serde_json::json;
use unimock::{MockFn, Unimock, matching};

use crate::{RunOutcome, StepOutcome, Turn};

use super::super::support::{TypedEchoTool, runtime, user_message};

#[tokio::test]
async fn tool_errors_surface_on_step_and_are_reused_by_run() {
    let model: Arc<dyn Model> = Arc::new(Unimock::new((
        ModelMock::complete
            .next_call(matching!(_))
            .returns(Ok(ModelResponse {
                provider: "mock".to_owned(),
                model_id: "mock-model".to_owned(),
                catalog_model_id: None,
                thinking_level: None,
                assistant_message: "trying tool".to_owned(),
                tool_calls: vec![ToolCall {
                    id: "call-1".to_owned(),
                    name: "echo".to_owned(),
                    input: json!({ "message": "hello" }),
                    operation_id: None,
                }],
                finish_reason: FinishReason::ToolCalls,
                structured_output: None,
                provider_context: Vec::new(),
                usage: ModelUsage::default(),
            })),
        ModelMock::complete
            .next_call(matching!(_))
            .answers(&|_, request: &ModelRequest| {
                let tool_message = request
                    .messages
                    .iter()
                    .find(|message| message.role == ai_interface::ConversationRole::Tool)
                    .expect("tool error should be retained");
                assert!(tool_message.content.contains("typed tool execution failed"));
                Ok(ModelResponse {
                    provider: "mock".to_owned(),
                    model_id: "mock-model".to_owned(),
                    catalog_model_id: None,
                    thinking_level: None,
                    assistant_message: "recovered".to_owned(),
                    tool_calls: Vec::new(),
                    finish_reason: FinishReason::Stop,
                    structured_output: None,
                    provider_context: Vec::new(),
                    usage: ModelUsage::default(),
                })
            }),
    )));
    let tool = TypedEchoTool::fail_once();
    let runtime = runtime(model, vec![tool.tool()]).expect("runtime should build");

    let mut turn = runtime.send(user_message("start"), Some(4));
    let error = turn.step().await.expect_err("first step should fail");
    assert!(matches!(error, crate::Error::Tool(_)));

    let outcome = turn.run().await.expect("run should recover");
    assert!(matches!(
        outcome,
        RunOutcome::Completed {
            assistant_message,
            steps_taken: 2,
        } if assistant_message == "recovered"
    ));
}

#[tokio::test]
async fn step_limit_is_reported_as_capped() {
    let model: Arc<dyn Model> = Arc::new(Unimock::new(
        ModelMock::complete
            .next_call(matching!(_))
            .returns(Ok(ModelResponse {
                provider: "mock".to_owned(),
                model_id: "mock-model".to_owned(),
                catalog_model_id: None,
                thinking_level: None,
                assistant_message: "still going".to_owned(),
                tool_calls: vec![ToolCall {
                    id: "call-1".to_owned(),
                    name: "echo".to_owned(),
                    input: json!({ "message": "hello" }),
                    operation_id: None,
                }],
                finish_reason: FinishReason::ToolCalls,
                structured_output: None,
                provider_context: Vec::new(),
                usage: ModelUsage::default(),
            })),
    ));
    let runtime =
        runtime(model, vec![TypedEchoTool::succeeding().tool()]).expect("runtime should build");

    let mut turn = runtime.send(user_message("start"), Some(1));
    let first_step = turn.step().await.expect("first step should succeed");
    assert!(matches!(
        first_step,
        StepOutcome::Stepped { steps_taken: 1, .. }
    ));

    let outcome = turn.run().await.expect("run should cap");
    assert!(matches!(
        outcome,
        RunOutcome::Capped {
            assistant_message,
            steps_taken: 1,
            max_steps: 1,
        } if assistant_message == "still going"
    ));
}

#[tokio::test]
async fn invalid_tool_arguments_surface_once_and_are_retained() {
    let model: Arc<dyn Model> = Arc::new(Unimock::new((
        ModelMock::complete
            .next_call(matching!(_))
            .returns(Ok(ModelResponse {
                provider: "mock".to_owned(),
                model_id: "mock-model".to_owned(),
                catalog_model_id: None,
                thinking_level: None,
                assistant_message: "tool call".to_owned(),
                tool_calls: vec![ToolCall {
                    id: "call-1".to_owned(),
                    name: "echo".to_owned(),
                    input: json!({ "invalid": true }),
                    operation_id: None,
                }],
                finish_reason: FinishReason::ToolCalls,
                structured_output: None,
                provider_context: Vec::new(),
                usage: ModelUsage::default(),
            })),
        ModelMock::complete
            .next_call(matching!(_))
            .answers(&|_, request: &ModelRequest| {
                let tool_message = request
                    .messages
                    .iter()
                    .find(|message| message.role == ai_interface::ConversationRole::Tool)
                    .expect("tool error should be retained");
                assert!(tool_message.content.contains("invalid arguments"));
                Ok(ModelResponse {
                    provider: "mock".to_owned(),
                    model_id: "mock-model".to_owned(),
                    catalog_model_id: None,
                    thinking_level: None,
                    assistant_message: "after parse failure".to_owned(),
                    tool_calls: Vec::new(),
                    finish_reason: FinishReason::Stop,
                    structured_output: None,
                    provider_context: Vec::new(),
                    usage: ModelUsage::default(),
                })
            }),
    )));
    let tool = TypedEchoTool::succeeding();
    let runtime = runtime(model, vec![tool.tool()]).expect("runtime should build");

    let mut turn = runtime.send(user_message("start"), Some(4));
    let error = turn
        .step()
        .await
        .expect_err("step should return parse error");
    assert!(matches!(
        error,
        crate::Error::Tool(ToolError::InvalidArguments { .. })
    ));
    assert_eq!(tool.parse_count(), 0);

    let outcome = turn.run().await.expect("run should continue");
    assert!(matches!(
        outcome,
        RunOutcome::Completed {
            assistant_message,
            steps_taken: 2,
        } if assistant_message == "after parse failure"
    ));
}

#[tokio::test]
async fn failed_tool_calls_still_append_messages_for_later_calls_in_the_same_batch() {
    let model: Arc<dyn Model> = Arc::new(Unimock::new((
        ModelMock::complete
            .next_call(matching!(_))
            .returns(Ok(ModelResponse {
                provider: "mock".to_owned(),
                model_id: "mock-model".to_owned(),
                catalog_model_id: None,
                thinking_level: None,
                assistant_message: "trying tools".to_owned(),
                tool_calls: vec![
                    ToolCall {
                        id: "call-1".to_owned(),
                        name: "echo".to_owned(),
                        input: json!({ "message": "first" }),
                        operation_id: None,
                    },
                    ToolCall {
                        id: "call-2".to_owned(),
                        name: "echo".to_owned(),
                        input: json!({ "message": "second" }),
                        operation_id: None,
                    },
                ],
                finish_reason: FinishReason::ToolCalls,
                structured_output: None,
                provider_context: Vec::new(),
                usage: ModelUsage::default(),
            })),
        ModelMock::complete
            .next_call(matching!(_))
            .answers(&|_, request: &ModelRequest| {
                let tool_messages = request
                    .messages
                    .iter()
                    .filter(|message| message.role == ai_interface::ConversationRole::Tool)
                    .cloned()
                    .collect::<Vec<_>>();
                assert_eq!(tool_messages.len(), 2);
                assert_eq!(tool_messages[0].tool_call_id.as_deref(), Some("call-1"),);
                assert_eq!(tool_messages[1].tool_call_id.as_deref(), Some("call-2"),);
                assert!(tool_messages[0].content.contains("\"ok\":false"));
                assert!(tool_messages[1].content.contains("\"echo\":\"second\""));
                Ok(ModelResponse {
                    provider: "mock".to_owned(),
                    model_id: "mock-model".to_owned(),
                    catalog_model_id: None,
                    thinking_level: None,
                    assistant_message: "recovered".to_owned(),
                    tool_calls: Vec::new(),
                    finish_reason: FinishReason::Stop,
                    structured_output: None,
                    provider_context: Vec::new(),
                    usage: ModelUsage::default(),
                })
            }),
    )));
    let tool = TypedEchoTool::fail_once();
    let runtime = runtime(model, vec![tool.tool()]).expect("runtime should build");

    let mut turn = runtime.send(user_message("start"), Some(4));
    let error = turn.step().await.expect_err("first step should fail");
    assert!(matches!(error, crate::Error::Tool(_)));
    assert_eq!(tool.parse_count(), 2);

    let outcome = turn.run().await.expect("run should recover");
    assert!(matches!(
        outcome,
        RunOutcome::Completed {
            assistant_message,
            steps_taken: 2,
        } if assistant_message == "recovered"
    ));
}
