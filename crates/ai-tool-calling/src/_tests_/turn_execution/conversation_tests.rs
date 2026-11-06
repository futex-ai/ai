//! Runtime tests through the model stream contract.

use std::sync::Arc;

use ai_interface::test_support::model_response_stream;
use ai_interface::{
    ConversationRole, FinishReason, Logger, LoggerMock, Model, ModelMock, ModelRequest,
    ModelResponse, ModelUsage, ToolActivityLogEntry, ToolActivityPhase, ToolCall, ToolCallLogEntry,
    ToolCallLogResult,
};
use serde_json::json;
use unimock::{MockFn, Unimock, matching};

use crate::tests::event_support::{collect_events, executions, request, run_outcome, steps};
use crate::tests::support::{response_parts, stream_result, tool_result_name};
use crate::{Agent, AgentEvent, RunOutcome};

use super::super::support::{TypedEchoTool, runtime, runtime_with_logger, user_message};

#[tokio::test]
async fn caller_owned_conversation_is_used_across_future_runs() {
    let model: Arc<dyn Model> = Arc::new(Unimock::new((
        ModelMock::stream
            .next_call(matching!(_))
            .answers_arc(stream_result(Ok(ModelResponse {
                attempts: Vec::new(),
                provider: "mock".to_owned(),
                model_id: "mock-model".to_owned(),
                catalog_model_id: None,
                thinking_level: None,
                parts: response_parts("first response".to_owned(), Vec::new(), Vec::new()),
                finish_reason: FinishReason::Stop,
                structured_output: None,
                usage: ModelUsage::default(),
            }))),
        ModelMock::stream
            .next_call(matching!(_))
            .answers(&|_, request: &ModelRequest| {
                {
                    assert!(request.messages.iter().any(|message| {
                        message.role == ConversationRole::Assistant
                            && message.text() == "first response"
                    }));
                    Ok(ModelResponse {
                        attempts: Vec::new(),
                        provider: "mock".to_owned(),
                        model_id: "mock-model".to_owned(),
                        catalog_model_id: None,
                        thinking_level: None,
                        parts: response_parts("second response".to_owned(), Vec::new(), Vec::new()),
                        finish_reason: FinishReason::Stop,
                        structured_output: None,
                        usage: ModelUsage::default(),
                    })
                }
                .map(model_response_stream)
            }),
    )));
    let runtime = runtime(model, Vec::new()).expect("runtime should build");

    let mut conversation = vec![user_message("first message")];
    collect_events(runtime.run(&mut conversation, request(Some(4))))
        .await
        .expect("first run should succeed");

    conversation.push(user_message("second message"));
    let events = collect_events(runtime.run(&mut conversation, request(Some(4))))
        .await
        .expect("second run should succeed");
    let outcome = run_outcome(&events);
    assert!(matches!(
        outcome,
        RunOutcome::Completed {
            assistant_message,
            steps_taken: 1,
            ..
        } if assistant_message == "second response"
    ));
}

#[tokio::test]
async fn step_and_run_handle_tool_rounds_and_logger_callbacks() {
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
                    "calling tool".to_owned(),
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
                    assert!(request.messages.iter().any(|message| {
                        message.role == ConversationRole::Assistant
                            && message.text() == "calling tool"
                            && message.tool_calls().iter().any(|call| call.id == "call-1")
                    }));
                    assert!(request.messages.iter().any(|message| {
                        message.role == ConversationRole::Tool
                            && tool_result_name(message) == Some("echo")
                            && message.text().contains("\"hello\"")
                    }));
                    Ok(ModelResponse {
                        attempts: Vec::new(),
                        provider: "mock".to_owned(),
                        model_id: "mock-model".to_owned(),
                        catalog_model_id: None,
                        thinking_level: None,
                        parts: response_parts("all done".to_owned(), Vec::new(), Vec::new()),
                        finish_reason: FinishReason::Stop,
                        structured_output: None,
                        usage: ModelUsage::default(),
                    })
                }
                .map(model_response_stream)
            }),
    )));
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
                assert_eq!(entry.call.name, "echo");
                assert!(matches!(entry.result, ToolCallLogResult::Success { .. }));
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
        LoggerMock::log_model_call
            .next_call(matching!(_))
            .returns(Ok(())),
        LoggerMock::log_turn_outcome
            .next_call(matching!(_))
            .returns(Ok(())),
    )));
    let tool = TypedEchoTool::succeeding();
    let runtime =
        runtime_with_logger(model, logger, vec![tool.tool()]).expect("runtime should build");

    let mut conversation = vec![user_message("start")];
    let mut events = Vec::new();
    let mut stream = runtime.run(&mut conversation, request(Some(4)));
    loop {
        let event = stream.next().await.unwrap().expect("first step event");
        let finished = matches!(&event, AgentEvent::StepFinished { .. });
        events.push(event);
        if finished {
            break;
        }
    }
    assert!(matches!(
        steps(&events).as_slice(),
        [messages] if messages[0].text() == "calling tool"
    ));
    assert_eq!(tool.parse_count(), 1);

    events.extend(collect_events(stream).await.expect("run should complete"));
    assert!(matches!(
        run_outcome(&events),
        RunOutcome::Completed {
            assistant_message,
            steps_taken: 2,
            ..
        } if assistant_message == "all done"
    ));
    assert_eq!(executions(&events).len(), 1);
}
