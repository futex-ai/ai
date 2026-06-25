use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};

use ai_interface::{
    ConversationRole, FinishReason, Model, ModelMock, ModelResponse, ModelUsage, ToolCall,
};
use serde_json::json;
use unimock::{MockFn, Unimock, matching};

use crate::{Error, StepOutcome, TurnCheckpointMock};

use super::super::support::{TypedEchoTool, runtime, user_message};

#[tokio::test]
async fn checkpoint_runs_before_each_tool_and_after_completed_tools() {
    let model: Arc<dyn Model> = Arc::new(Unimock::new(
        ModelMock::complete
            .next_call(matching!(_))
            .returns(Ok(two_tool_response())),
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
    let mut turn = runtime.send(user_message("start"), Some(4));

    let outcome = turn
        .step_with_checkpoint(&mut checkpoint)
        .await
        .expect("step should interrupt cleanly");

    assert!(matches!(
        outcome,
        StepOutcome::Stepped { tool_results, .. } if tool_results.len() == 2
    ));
    assert_eq!(tool.parse_count(), 2);
    let conversation = runtime.conversation();
    let tool_messages = conversation
        .iter()
        .filter(|message| message.tool_call_id.is_some())
        .collect::<Vec<_>>();
    assert_eq!(tool_messages.len(), 2);
    assert!(tool_messages[0].content.contains("\"echo\":\"first\""));
    assert!(tool_messages[1].content.contains("\"echo\":\"second\""));
    assert_eq!(
        conversation
            .last()
            .and_then(|message| message.tool_call_id.as_deref()),
        Some("call-2")
    );
}

#[tokio::test]
async fn checkpoint_error_before_model_call_stops_without_model_request() {
    let model: Arc<dyn Model> = Arc::new(Unimock::new(()));
    let runtime = runtime(model, Vec::new()).expect("runtime should build");
    let mut checkpoint = Unimock::new(TurnCheckpointMock::check.next_call(matching!()).returns(
        Err(Error::checkpoint(std::io::Error::other("agent killed"))),
    ));
    let mut turn = runtime.send(user_message("start"), Some(4));

    let error = turn
        .step_with_checkpoint(&mut checkpoint)
        .await
        .expect_err("checkpoint error should stop the step");

    assert!(matches!(error, Error::Checkpoint { .. }));
    assert!(
        runtime
            .conversation()
            .iter()
            .all(|message| message.role != ConversationRole::Assistant)
    );
}

#[tokio::test]
async fn checkpoint_error_after_terminal_model_call_stops_before_assistant_message() {
    let model_calls = Arc::new(AtomicUsize::new(0));
    let model: Arc<dyn Model> = Arc::new(Unimock::new(
        ModelMock::complete.each_call(matching!(_)).answers_arc({
            let model_calls = model_calls.clone();
            Arc::new(move |_, _| {
                model_calls.fetch_add(1, Ordering::SeqCst);
                Ok(terminal_response())
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
    let mut turn = runtime.send(user_message("start"), Some(4));

    let error = turn
        .step_with_checkpoint(&mut checkpoint)
        .await
        .expect_err("checkpoint error should stop before storing terminal response");

    assert!(matches!(error, Error::Checkpoint { .. }));
    assert_eq!(model_calls.load(Ordering::SeqCst), 1);
    assert!(
        runtime
            .conversation()
            .iter()
            .all(|message| message.role != ConversationRole::Assistant)
    );
}

fn two_tool_response() -> ModelResponse {
    ModelResponse {
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
    }
}

fn terminal_response() -> ModelResponse {
    ModelResponse {
        provider: "mock".to_owned(),
        model_id: "mock-model".to_owned(),
        catalog_model_id: None,
        thinking_level: None,
        assistant_message: "done".to_owned(),
        tool_calls: Vec::new(),
        finish_reason: FinishReason::Stop,
        structured_output: None,
        provider_context: Vec::new(),
        usage: ModelUsage::default(),
    }
}
