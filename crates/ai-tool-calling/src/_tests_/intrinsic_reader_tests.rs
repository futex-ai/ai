//! Runtime tests through the model stream contract.

use std::sync::Arc;

use ai_interface::test_support::model_response_stream;
use ai_interface::{
    ConversationRole, FinishReason, Logger, LoggerMock, Model, ModelMock, ModelRequest,
    ModelResponse, ModelUsage, ToolActivityLogEntry, ToolActivityPhase, ToolCall, ToolCallLogEntry,
    ToolCallLogResult, ToolOutputEnvelope,
};
use serde_json::json;
use unimock::{MockFn, Unimock, matching};

use crate::tests::event_support::{collect_events, executions, request, run_outcome};
use crate::tests::support::{response_parts, stream_result};
use crate::{
    Agent, InMemoryToolOutputStore, ToolOutputPolicy, ToolOutputStore, ToolOutputWriteRequest,
};

use super::support::{runtime_with_logger_store_and_policy, user_message};

#[tokio::test]
async fn intrinsic_reader_reads_requested_id_without_recursive_wrapping() {
    let store = Arc::new(InMemoryToolOutputStore::new());
    let policy = ToolOutputPolicy::new(5, 5, 100, 100).unwrap();
    let written = store
        .write(ToolOutputWriteRequest {
            tool_name: "search".to_owned(),
            content: "abcdefghij".to_owned(),
            policy,
            first_window_length: 5,
        })
        .await
        .unwrap();
    let output_id = written.output_id.clone();
    let model: Arc<dyn Model> = Arc::new(Unimock::new(
        ModelMock::stream
            .next_call(matching!(_))
            .answers_arc(stream_result(Ok(tool_call_response(ToolCall {
                id: "read-call".to_owned(),
                name: "tool_output_read".to_owned(),
                input: json!({ "output_id": output_id.clone(), "offset": 5 }),
                operation_id: Some("read-operation".to_owned()),
            })))),
    ));
    let logger: Arc<dyn Logger> = Arc::new(Unimock::new((
        LoggerMock::log_model_call
            .next_call(matching!(_))
            .returns(Ok(())),
        LoggerMock::log_tool_activity
            .next_call(matching!(_))
            .answers(&|_, entry: &ToolActivityLogEntry| {
                assert_eq!(entry.tool_name, "tool_output_read");
                assert_eq!(entry.phase, ToolActivityPhase::Started);
                Ok(())
            }),
        LoggerMock::log_tool_call.next_call(matching!(_)).answers(
            &|_, entry: &ToolCallLogEntry| {
                assert!(matches!(
                    &entry.result,
                    ToolCallLogResult::Success {
                        output: ToolOutputEnvelope::Window(window)
                    } if window.output_id().is_some()
                ));
                Ok(())
            },
        ),
        LoggerMock::log_tool_activity
            .next_call(matching!(_))
            .answers(&|_, entry: &ToolActivityLogEntry| {
                assert_eq!(entry.tool_name, "tool_output_read");
                assert_eq!(entry.phase, ToolActivityPhase::Completed);
                Ok(())
            }),
        LoggerMock::log_turn_outcome
            .some_call(matching!(_))
            .returns(Ok(())),
    )));
    let runtime = runtime_with_logger_store_and_policy(model, logger, Vec::new(), store, policy)
        .expect("runtime should build");

    let mut conversation = vec![user_message("start")];
    let events = collect_events(runtime.run(&mut conversation, request(Some(1))))
        .await
        .expect("read should succeed");

    assert!(matches!(
        executions(&events).as_slice(),
        [record] if record.output_id.as_ref() == Some(&output_id)
            && record.operation_id == "read-operation"
            && record.raw_output
                == serde_json::to_value(&record.model_visible_output).unwrap()
    ));
}

#[tokio::test]
async fn replaced_output_store_makes_previous_ids_unavailable_and_run_recovers() {
    let old_store = Arc::new(InMemoryToolOutputStore::new());
    let policy = ToolOutputPolicy::new(5, 5, 100, 100).unwrap();
    let written = old_store
        .write(ToolOutputWriteRequest {
            tool_name: "search".to_owned(),
            content: "abcdefghij".to_owned(),
            policy,
            first_window_length: 5,
        })
        .await
        .unwrap();
    let output_id = written.output_id.clone();
    let model: Arc<dyn Model> = Arc::new(Unimock::new((
        ModelMock::stream
            .next_call(matching!(_))
            .answers_arc(stream_result(Ok(tool_call_response(ToolCall {
                id: "read-call".to_owned(),
                name: "tool_output_read".to_owned(),
                input: json!({ "output_id": output_id }),
                operation_id: None,
            })))),
        ModelMock::stream
            .next_call(matching!(_))
            .answers(&|_, request: &ModelRequest| {
                {
                    let content = request
                        .messages
                        .iter()
                        .find(|message| message.role == ConversationRole::Tool)
                        .expect("tool error should be retained")
                        .text()
                        .clone();
                    assert!(content.contains("\"ok\":false"));
                    assert!(content.contains("no longer available"));
                    assert!(content.contains("the original tool call itself succeeded"));
                    assert!(content.contains("confirm with the user"));
                    assert!(!content.contains("original tool call failed"));
                    assert!(!content.contains("[ai_interface/tool]"));
                    assert!(!content.contains("[ai_tool_calling/output_store]"));
                    Ok(stop_response("recovered"))
                }
                .map(model_response_stream)
            }),
    )));
    let logger: Arc<dyn Logger> = Arc::new(Unimock::new((
        LoggerMock::log_model_call
            .each_call(matching!(_))
            .answers(&|_, _| Ok(())),
        LoggerMock::log_tool_activity
            .each_call(matching!(_))
            .answers(&|_, _| Ok(())),
        LoggerMock::log_tool_call.next_call(matching!(_)).answers(
            &|_, entry: &ToolCallLogEntry| {
                assert!(matches!(
                    &entry.result,
                    ToolCallLogResult::Error { message, debug }
                        if message.contains("[ai_interface/tool]")
                            && message.contains("[ai_tool_calling/output_store]")
                            && debug.contains("UnavailableOutput")
                ));
                Ok(())
            },
        ),
        LoggerMock::log_turn_outcome
            .next_call(matching!(_))
            .returns(Ok(())),
    )));
    let runtime =
        runtime_with_logger_store_and_policy(model, logger, Vec::new(), old_store, policy)
            .expect("runtime should build");
    runtime.replace_output_store(Arc::new(InMemoryToolOutputStore::new()));

    let mut conversation = vec![user_message("start")];
    let events = collect_events(runtime.run(&mut conversation, request(Some(3))))
        .await
        .expect("run should recover");
    let outcome = run_outcome(&events);

    assert!(matches!(
        outcome,
        crate::RunOutcome::Completed {
            assistant_message,
            steps_taken: 2,
            ..
        } if assistant_message == "recovered"
    ));
}

fn tool_call_response(call: ToolCall) -> ModelResponse {
    ModelResponse {
        attempts: Vec::new(),
        provider: "mock".to_owned(),
        model_id: "mock-model".to_owned(),
        catalog_model_id: None,
        thinking_level: None,
        parts: response_parts("calling tool".to_owned(), vec![call], Vec::new()),
        finish_reason: FinishReason::ToolCalls,
        structured_output: None,
        usage: ModelUsage::default(),
    }
}

fn stop_response(message: &str) -> ModelResponse {
    ModelResponse {
        attempts: Vec::new(),
        provider: "mock".to_owned(),
        model_id: "mock-model".to_owned(),
        catalog_model_id: None,
        thinking_level: None,
        parts: response_parts(message.to_owned(), Vec::new(), Vec::new()),
        finish_reason: FinishReason::Stop,
        structured_output: None,
        usage: ModelUsage::default(),
    }
}
