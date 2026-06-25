use std::sync::Arc;

use ai_interface::{
    FinishReason, Logger, LoggerMock, Model, ModelCallLogEntry, ModelCallLogResult, ModelError,
    ModelMock, ModelResponse, ModelUsage, ToolActivityLogEntry, ToolActivityPhase, ToolCall,
    ToolCallLogEntry, ToolCallLogResult,
};
use serde_json::json;
use unimock::{MockFn, Unimock, matching};

use crate::Turn;

use super::support::{TypedEchoTool, runtime_with_logger, user_message};

#[tokio::test]
async fn model_errors_are_logged_before_the_turn_aborts() {
    #[derive(Debug, thiserror::Error)]
    #[error("[ai_tool_calling/tests] model failed")]
    struct FixtureModelError;

    let model: Arc<dyn Model> = Arc::new(Unimock::new(
        ModelMock::complete
            .next_call(matching!(_))
            .returns(Err(ModelError::internal(FixtureModelError))),
    ));
    let logger: Arc<dyn Logger> = Arc::new(Unimock::new(
        LoggerMock::log_model_call.next_call(matching!(_)).answers(
            &|_, entry: &ModelCallLogEntry| {
                assert!(matches!(
                    &entry.result,
                    ModelCallLogResult::Error { message, debug }
                        if message.contains("model failed") && debug.contains("FixtureModelError")
                ));
                Ok(())
            },
        ),
    ));
    let runtime = runtime_with_logger(model, logger, Vec::new()).expect("runtime should build");

    let mut turn = runtime.send(user_message("start"), Some(4));
    let error = turn.step().await.expect_err("model error should abort");

    assert!(matches!(error, crate::Error::Model(_)));
}

#[tokio::test]
async fn tool_errors_include_debug_details_in_logger_callbacks() {
    let model: Arc<dyn Model> = Arc::new(Unimock::new(
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
                usage: ModelUsage::default(),
            })),
    ));
    let logger: Arc<dyn Logger> = Arc::new(Unimock::new((
        LoggerMock::log_model_call
            .next_call(matching!(_))
            .returns(Ok(())),
        LoggerMock::log_tool_activity
            .next_call(matching!(_))
            .answers(&|_, entry: &ToolActivityLogEntry| {
                assert_eq!(entry.tool_name, "echo");
                assert_eq!(entry.activity_verb.as_deref(), Some("Echoing"));
                assert_eq!(entry.phase, ToolActivityPhase::Started);
                Ok(())
            }),
        LoggerMock::log_tool_call.next_call(matching!(_)).answers(
            &|_, entry: &ToolCallLogEntry| {
                assert!(matches!(
                    &entry.result,
                    ToolCallLogResult::Error { message, debug }
                        if message.contains("typed tool execution failed")
                            && debug.contains("FixtureToolError")
                ));
                Ok(())
            },
        ),
        LoggerMock::log_tool_activity
            .next_call(matching!(_))
            .answers(&|_, entry: &ToolActivityLogEntry| {
                assert_eq!(entry.tool_name, "echo");
                assert_eq!(entry.activity_verb.as_deref(), Some("Echoing"));
                assert_eq!(entry.phase, ToolActivityPhase::Completed);
                Ok(())
            }),
    )));
    let tool = TypedEchoTool::fail_once();
    let runtime =
        runtime_with_logger(model, logger, vec![tool.tool()]).expect("runtime should build");

    let mut turn = runtime.send(user_message("start"), Some(4));
    let error = turn.step().await.expect_err("tool error should surface");

    assert!(matches!(error, crate::Error::Tool(_)));
}
