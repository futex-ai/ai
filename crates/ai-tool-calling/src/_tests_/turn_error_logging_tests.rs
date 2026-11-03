//! Runtime tests through the model stream contract.

use std::sync::Arc;

use ai_interface::{
    ErrorCore, ErrorKind, FinishReason, Logger, LoggerMock, Model, ModelCallLogEntry,
    ModelCallLogResult, ModelError, ModelMock, ModelResponse, ModelUsage, SafeErrorSource,
    ToolActivityLogEntry, ToolActivityPhase, ToolCall, ToolCallLogEntry, ToolCallLogResult,
};
use serde_json::json;
use unimock::{MockFn, Unimock, matching};

use crate::Agent;
use crate::tests::event_support::{collect_events, request, tool_errors};
use crate::tests::support::{response_parts, stream_result};

use super::support::{TypedEchoTool, runtime_with_logger, user_message};

#[tokio::test]
async fn model_errors_are_logged_before_the_turn_aborts() {
    #[derive(Debug, thiserror::Error)]
    #[error("[ai_tool_calling/tests] model failed")]
    struct FixtureModelError;

    let model: Arc<dyn Model> = Arc::new(Unimock::new(
        ModelMock::stream
            .next_call(matching!(_))
            .answers_arc(stream_result(Err(ModelError::with_source(
                ErrorCore::new(ErrorKind::Internal, "mock", "mock-model", "model failed"),
                SafeErrorSource::redacted(FixtureModelError),
            )))),
    ));
    let logger: Arc<dyn Logger> = Arc::new(Unimock::new(
        LoggerMock::log_model_call.next_call(matching!(_)).answers(
            &|_, entry: &ModelCallLogEntry| {
                assert!(matches!(
                    &entry.result,
                    ModelCallLogResult::Error { message, debug }
                        if message.contains("model failed") && debug.contains("model failed")
                ));
                Ok(())
            },
        ),
    ));
    let runtime = runtime_with_logger(model, logger, Vec::new()).expect("runtime should build");

    let mut conversation = vec![user_message("start")];
    let error = collect_events(runtime.run(&mut conversation, request(Some(4))))
        .await
        .expect_err("model error should abort");

    assert!(matches!(error, crate::Error::Model(_)));
    let crate::Error::Model(ModelError::Internal { core, source, .. }) = error else {
        panic!("expected the original model source");
    };
    assert_eq!(core.kind, ErrorKind::Internal);
    assert!(
        source
            .raw_cause()
            .downcast_ref::<FixtureModelError>()
            .is_some()
    );
}

#[tokio::test]
async fn tool_errors_include_debug_details_in_logger_callbacks() {
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
        LoggerMock::log_turn_outcome
            .some_call(matching!(_))
            .returns(Ok(())),
    )));
    let tool = TypedEchoTool::fail_once();
    let runtime =
        runtime_with_logger(model, logger, vec![tool.tool()]).expect("runtime should build");

    let mut conversation = vec![user_message("start")];
    let events = collect_events(runtime.run(&mut conversation, request(Some(1))))
        .await
        .expect("tool error should not fail the turn");

    assert_eq!(tool_errors(&events).len(), 1);
}
