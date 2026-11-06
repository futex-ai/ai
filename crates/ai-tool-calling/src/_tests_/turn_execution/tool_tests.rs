//! Runtime tests through the model stream contract.

use std::sync::Arc;

use ai_interface::test_support::model_response_stream;
use ai_interface::{
    FinishReason, Model, ModelMock, ModelRequest, ModelResponse, ModelUsage, ToolCall, ToolError,
};
use serde_json::json;
use unimock::{MockFn, Unimock, matching};

use crate::tests::event_support::{collect_events, request, run_outcome, steps, tool_errors};
use crate::tests::support::{response_parts, stream_result};
use crate::{Agent, RunOutcome};

use super::super::support::{TypedEchoTool, runtime, user_message};

#[tokio::test]
async fn tool_errors_emit_finished_events_and_the_run_recovers() {
    let model: Arc<dyn Model> = Arc::new(Unimock::new((
        ModelMock::stream
            .next_call(matching!(_))
            .answers_arc(stream_result(Ok(ModelResponse {
                attempts: Vec::new(),
                provider: "mock".to_owned(),
                model_id: "mock-model".to_owned(),
                catalog_model_id: None,
                thinking_level: None,
                parts: response_parts(
                    "trying tool".to_owned(),
                    vec![ToolCall {
                        id: "call-1".to_owned(),
                        name: "echo".to_owned(),
                        input: json!({ "message": "hello" }),
                        operation_id: None,
                    }],
                    Vec::new(),
                ),
                finish_reason: FinishReason::ToolCalls,
                structured_output: None,
                usage: ModelUsage::default(),
            }))),
        ModelMock::stream
            .next_call(matching!(_))
            .answers(&|_, request: &ModelRequest| {
                {
                    let tool_message = request
                        .messages
                        .iter()
                        .find(|message| message.role == ai_interface::ConversationRole::Tool)
                        .expect("tool error should be retained");
                    assert!(tool_message.text().contains("typed tool execution failed"));
                    Ok(ModelResponse {
                        attempts: Vec::new(),
                        provider: "mock".to_owned(),
                        model_id: "mock-model".to_owned(),
                        catalog_model_id: None,
                        thinking_level: None,
                        parts: response_parts("recovered".to_owned(), Vec::new(), Vec::new()),
                        finish_reason: FinishReason::Stop,
                        structured_output: None,
                        usage: ModelUsage::default(),
                    })
                }
                .map(model_response_stream)
            }),
    )));
    let tool = TypedEchoTool::fail_once();
    let runtime = runtime(model, vec![tool.tool()]).expect("runtime should build");

    let mut conversation = vec![user_message("start")];
    let events = collect_events(runtime.run(&mut conversation, request(Some(4))))
        .await
        .expect("tool error should not stop the turn");
    assert!(matches!(
        tool_errors(&events).as_slice(),
        [ToolError::Execution { .. }]
    ));

    let outcome = run_outcome(&events);
    assert!(matches!(
        outcome,
        RunOutcome::Completed {
            assistant_message,
            steps_taken: 2,
            ..
        } if assistant_message == "recovered"
    ));
}

#[tokio::test]
async fn step_limit_is_reported_as_capped() {
    let model: Arc<dyn Model> = Arc::new(Unimock::new(
        ModelMock::stream
            .next_call(matching!(_))
            .answers_arc(stream_result(Ok(ModelResponse {
                attempts: Vec::new(),
                provider: "mock".to_owned(),
                model_id: "mock-model".to_owned(),
                catalog_model_id: None,
                thinking_level: None,
                parts: response_parts(
                    "still going".to_owned(),
                    vec![ToolCall {
                        id: "call-1".to_owned(),
                        name: "echo".to_owned(),
                        input: json!({ "message": "hello" }),
                        operation_id: None,
                    }],
                    Vec::new(),
                ),
                finish_reason: FinishReason::ToolCalls,
                structured_output: None,
                usage: ModelUsage::default(),
            }))),
    ));
    let runtime =
        runtime(model, vec![TypedEchoTool::succeeding().tool()]).expect("runtime should build");

    let mut conversation = vec![user_message("start")];
    let events = collect_events(runtime.run(&mut conversation, request(Some(1))))
        .await
        .expect("run should cap");
    assert_eq!(steps(&events).len(), 1);

    let outcome = run_outcome(&events);
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
        ModelMock::stream
            .next_call(matching!(_))
            .answers_arc(stream_result(Ok(ModelResponse {
                attempts: Vec::new(),
                provider: "mock".to_owned(),
                model_id: "mock-model".to_owned(),
                catalog_model_id: None,
                thinking_level: None,
                parts: response_parts(
                    "tool call".to_owned(),
                    vec![ToolCall {
                        id: "call-1".to_owned(),
                        name: "echo".to_owned(),
                        input: json!({ "invalid": true }),
                        operation_id: None,
                    }],
                    Vec::new(),
                ),
                finish_reason: FinishReason::ToolCalls,
                structured_output: None,
                usage: ModelUsage::default(),
            }))),
        ModelMock::stream
            .next_call(matching!(_))
            .answers(&|_, request: &ModelRequest| {
                {
                    let tool_message = request
                        .messages
                        .iter()
                        .find(|message| message.role == ai_interface::ConversationRole::Tool)
                        .expect("tool error should be retained");
                    assert!(tool_message.text().contains("invalid arguments"));
                    Ok(ModelResponse {
                        attempts: Vec::new(),
                        provider: "mock".to_owned(),
                        model_id: "mock-model".to_owned(),
                        catalog_model_id: None,
                        thinking_level: None,
                        parts: response_parts(
                            "after parse failure".to_owned(),
                            Vec::new(),
                            Vec::new(),
                        ),
                        finish_reason: FinishReason::Stop,
                        structured_output: None,
                        usage: ModelUsage::default(),
                    })
                }
                .map(model_response_stream)
            }),
    )));
    let tool = TypedEchoTool::succeeding();
    let runtime = runtime(model, vec![tool.tool()]).expect("runtime should build");

    let mut conversation = vec![user_message("start")];
    let events = collect_events(runtime.run(&mut conversation, request(Some(4))))
        .await
        .expect("invalid arguments should not stop the turn");
    assert!(matches!(
        tool_errors(&events).as_slice(),
        [ToolError::InvalidArguments { .. }]
    ));
    assert_eq!(tool.parse_count(), 0);

    let outcome = run_outcome(&events);
    assert!(matches!(
        outcome,
        RunOutcome::Completed {
            assistant_message,
            steps_taken: 2,
            ..
        } if assistant_message == "after parse failure"
    ));
}
