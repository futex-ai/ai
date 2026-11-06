//! Runtime tests through the model stream contract.

use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};

use ai_interface::test_support::model_response_stream;
use ai_interface::{
    ConversationRole, FinishReason, Model, ModelMock, ModelResponse, ModelUsage, ToolCall,
};
use serde_json::json;
use unimock::{MockFn, Unimock, matching};

use crate::tests::event_support::{collect_events, executions, request};
use crate::tests::support::{response_parts, stream_result, tool_result_id};
use crate::{Agent, Error, NoopModelResponseCheckpoint, TurnCheckpointMock};

use super::super::support::{TypedEchoTool, runtime, user_message};

#[tokio::test]
async fn checkpoint_runs_before_tools_and_after_completed_tools() {
    let model: Arc<dyn Model> = Arc::new(Unimock::new(
        ModelMock::stream
            .next_call(matching!(_))
            .answers_arc(stream_result(Ok(two_tool_response()))),
    ));
    let tool = TypedEchoTool::succeeding();
    let runtime = runtime(model, vec![tool.tool()]).expect("runtime should build");
    let mut checkpoint = Unimock::new((
        TurnCheckpointMock::check
            .next_call(matching!())
            .returns(Ok(())),
        TurnCheckpointMock::check
            .next_call(matching!())
            .returns(Ok(())),
        TurnCheckpointMock::check
            .next_call(matching!())
            .returns(Ok(())),
        TurnCheckpointMock::check
            .next_call(matching!())
            .returns(Ok(())),
        TurnCheckpointMock::check
            .next_call(matching!())
            .returns(Ok(())),
    ));
    let mut conversation = vec![user_message("start")];
    let mut response_checkpoint = NoopModelResponseCheckpoint;

    let events = collect_events(runtime.run_with_checkpoints(
        &mut conversation,
        request(Some(1)),
        &mut checkpoint,
        &mut response_checkpoint,
    ))
    .await
    .expect("step should finish cleanly");

    assert_eq!(executions(&events).len(), 2);
    assert_eq!(tool.parse_count(), 2);
    let tool_messages = conversation
        .iter()
        .filter(|message| tool_result_id(message).is_some())
        .collect::<Vec<_>>();
    assert_eq!(tool_messages.len(), 2);
    assert!(tool_messages[0].text().contains("\"echo\":\"first\""));
    assert!(tool_messages[1].text().contains("\"echo\":\"second\""));
    assert_eq!(conversation.last().and_then(tool_result_id), Some("call-2"));
}

#[tokio::test]
async fn checkpoint_error_before_model_call_stops_without_model_request() {
    let model: Arc<dyn Model> = Arc::new(Unimock::new(()));
    let runtime = runtime(model, Vec::new()).expect("runtime should build");
    let mut checkpoint = Unimock::new(TurnCheckpointMock::check.next_call(matching!()).returns(
        Err(Error::checkpoint(std::io::Error::other("agent killed"))),
    ));
    let mut conversation = vec![user_message("start")];
    let mut response_checkpoint = NoopModelResponseCheckpoint;

    let error = collect_events(runtime.run_with_checkpoints(
        &mut conversation,
        request(Some(4)),
        &mut checkpoint,
        &mut response_checkpoint,
    ))
    .await
    .expect_err("checkpoint error should stop the step");

    assert!(matches!(error, Error::Checkpoint { .. }));
    assert!(
        conversation
            .iter()
            .all(|message| message.role != ConversationRole::Assistant)
    );
}

#[tokio::test]
async fn checkpoint_error_after_terminal_model_call_stops_before_assistant_message() {
    let model_calls = Arc::new(AtomicUsize::new(0));
    let model: Arc<dyn Model> = Arc::new(Unimock::new(
        ModelMock::stream.each_call(matching!(_)).answers_arc({
            let model_calls = model_calls.clone();
            Arc::new(move |_, _| {
                {
                    model_calls.fetch_add(1, Ordering::SeqCst);
                    Ok(terminal_response())
                }
                .map(model_response_stream)
            })
        }),
    ));
    let runtime = runtime(model, Vec::new()).expect("runtime should build");
    let mut checkpoint = Unimock::new((
        TurnCheckpointMock::check
            .next_call(matching!())
            .returns(Ok(())),
        TurnCheckpointMock::check
            .next_call(matching!())
            .returns(Err(Error::checkpoint(std::io::Error::other(
                "agent killed",
            )))),
    ));
    let mut conversation = vec![user_message("start")];
    let mut response_checkpoint = NoopModelResponseCheckpoint;

    let error = collect_events(runtime.run_with_checkpoints(
        &mut conversation,
        request(Some(4)),
        &mut checkpoint,
        &mut response_checkpoint,
    ))
    .await
    .expect_err("checkpoint error should stop before storing terminal response");

    assert!(matches!(error, Error::Checkpoint { .. }));
    assert_eq!(model_calls.load(Ordering::SeqCst), 1);
    assert!(
        conversation
            .iter()
            .all(|message| message.role != ConversationRole::Assistant)
    );
}

fn two_tool_response() -> ModelResponse {
    ModelResponse {
        attempts: Vec::new(),
        provider: "mock".to_owned(),
        model_id: "mock-model".to_owned(),
        catalog_model_id: None,
        thinking_level: None,
        parts: response_parts(
            "trying tools".to_owned(),
            vec![
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
            Vec::new(),
        ),
        finish_reason: FinishReason::ToolCalls,
        structured_output: None,
        usage: ModelUsage::default(),
    }
}

fn terminal_response() -> ModelResponse {
    ModelResponse {
        attempts: Vec::new(),
        provider: "mock".to_owned(),
        model_id: "mock-model".to_owned(),
        catalog_model_id: None,
        thinking_level: None,
        parts: response_parts("done".to_owned(), Vec::new(), Vec::new()),
        finish_reason: FinishReason::Stop,
        structured_output: None,
        usage: ModelUsage::default(),
    }
}
