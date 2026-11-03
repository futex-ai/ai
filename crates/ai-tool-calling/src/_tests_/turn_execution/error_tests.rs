//! Runtime tests through the model stream contract.

use std::sync::Arc;

use ai_interface::{
    DynModelEventStream, ErrorKind, FinishReason, Model, ModelAttempt, ModelError, ModelEvent,
    ModelEventStreamMock, ModelMock, ModelResponse, ModelUsage, SafeErrorSource, ToolCall,
};
use serde_json::json;
use unimock::{MockFn, Unimock, matching};

use crate::tests::event_support::{collect_events, executions, request, run_outcome, steps};
use crate::tests::support::{response_parts, stream_result};
use crate::{Agent, AgentEvent, RunOutcome};

use super::super::support::{TypedEchoTool, runtime, user_message};

#[tokio::test]
async fn model_errors_abort_the_turn() {
    #[derive(Debug, thiserror::Error)]
    #[error("[ai_tool_calling/tests] model failed")]
    struct FixtureModelError;

    let model: Arc<dyn Model> = Arc::new(Unimock::new(
        ModelMock::stream
            .next_call(matching!(_))
            .answers_arc(stream_result(Err(ModelError::internal(
                "mock",
                "mock-model",
                SafeErrorSource::redacted(FixtureModelError),
            )))),
    ));
    let runtime = runtime(model, Vec::new()).expect("runtime should build");

    let mut conversation = vec![user_message("start")];
    let error = collect_events(runtime.run(&mut conversation, request(Some(4))))
        .await
        .expect_err("model error should abort");
    assert!(matches!(error, crate::Error::Model(_)));
}

#[tokio::test]
async fn stop_finish_reason_completes_the_turn() {
    let model = single_response_model(model_response(FinishReason::Stop, Vec::new()));
    let runtime = runtime(model, Vec::new()).expect("runtime should build");

    let mut conversation = vec![user_message("start")];
    let events = collect_events(runtime.run(&mut conversation, request(Some(4))))
        .await
        .expect("turn should complete");
    let outcome = run_outcome(&events);

    assert!(matches!(
        outcome,
        RunOutcome::Completed {
            assistant_message,
            steps_taken: 1,
            ..
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

    let mut conversation = vec![user_message("start")];
    let events = collect_events(runtime.run(&mut conversation, request(Some(1))))
        .await
        .expect("step should dispatch tool");

    assert_eq!(steps(&events).len(), 1);
    assert_eq!(executions(&events).len(), 1);
}

#[tokio::test]
async fn tool_calls_finish_reason_without_calls_is_a_model_error() {
    let model = Arc::new(Unimock::new(
        ModelMock::stream.next_call(matching!(_)).answers(&|_, _| {
            let stream: DynModelEventStream = Box::new(Unimock::new((
                ModelEventStreamMock::next
                    .next_call(matching!())
                    .answers(&|_| {
                        Ok(Some(ModelEvent::TextDelta {
                            part_index: 0,
                            delta: "done".to_owned(),
                            provider_state: None,
                        }))
                    }),
                ModelEventStreamMock::next
                    .next_call(matching!())
                    .answers(&|_| {
                        Err(ModelError::invalid_output(
                            "mock",
                            "mock-model",
                            "tool-call finish requires at least one complete tool call",
                        ))
                    }),
            )));
            Ok(stream)
        }),
    ));
    let runtime = runtime(model, Vec::new()).expect("runtime should build");

    let mut conversation = vec![user_message("start")];
    let error = collect_events(runtime.run(&mut conversation, request(Some(4))))
        .await
        .expect_err("step should reject response");

    assert!(matches!(
        error,
        crate::Error::Model(error) if error.core().kind == ErrorKind::InvalidOutput
    ));
}

#[tokio::test]
async fn non_tool_finish_reason_discards_calls_without_dispatch() {
    let model = single_response_model(model_response(FinishReason::Stop, vec![echo_tool_call()]));
    let runtime = runtime(model, Vec::new()).expect("runtime should build");
    let mut conversation = vec![user_message("start")];
    let events = collect_events(runtime.run(&mut conversation, request(Some(4))))
        .await
        .expect("shared fold should discard calls on stop");

    assert!(matches!(
        run_outcome(&events),
        RunOutcome::Completed { steps_taken: 1, .. }
    ));
    assert!(
        events
            .iter()
            .all(|event| !matches!(event, AgentEvent::ToolStarted { .. }))
    );
    assert!(
        conversation
            .iter()
            .all(|message| message.tool_calls().is_empty())
    );
}

#[tokio::test]
async fn truncated_finish_reason_ends_the_turn_without_completion() {
    let model = single_response_model(model_response(FinishReason::Truncated, Vec::new()));
    let runtime = runtime(model, Vec::new()).expect("runtime should build");
    let mut conversation = vec![user_message("start")];
    let mut stream = runtime.run(&mut conversation, request(Some(1)));
    let outcome = loop {
        let event = stream.next().await.unwrap().expect("terminal event");
        if let AgentEvent::Finished(outcome) = event {
            break outcome;
        }
    };
    assert_eq!(
        outcome,
        RunOutcome::Truncated {
            assistant_message: "done".to_owned(),
            steps_taken: 1,
        }
    );
    assert!(stream.next().await.unwrap().is_none());
    assert!(stream.next().await.unwrap().is_none());
}

#[tokio::test]
async fn filtered_finish_reason_is_a_model_error() {
    let mut response = model_response(FinishReason::Filtered, Vec::new());
    response.attempts.push(ModelAttempt {
        provider: "first".to_owned(),
        model_id: "first-model".to_owned(),
        kind: ErrorKind::RejectedKey,
        message: "key rejected".to_owned(),
        emitted_events: false,
    });
    let expected_attempts = response.attempts.clone();
    let model = single_response_model(response);
    let runtime = runtime(model, Vec::new()).expect("runtime should build");

    let mut conversation = vec![user_message("start")];
    let error = collect_events(runtime.run(&mut conversation, request(Some(4))))
        .await
        .expect_err("step should reject response");

    let crate::Error::Model(error) = error else {
        panic!("expected a model error");
    };
    assert_eq!(error.core().kind, ErrorKind::InvalidOutput);
    assert_eq!(error.core().provider, "mock");
    assert_eq!(error.core().model_id, "mock-model");
    assert_eq!(error.attempts(), expected_attempts);
}

#[tokio::test]
async fn other_finish_reason_is_a_model_error_with_raw_reason() {
    let model = single_response_model(model_response(
        FinishReason::Other("custom".to_owned()),
        Vec::new(),
    ));
    let runtime = runtime(model, Vec::new()).expect("runtime should build");

    let mut conversation = vec![user_message("start")];
    let error = collect_events(runtime.run(&mut conversation, request(Some(4))))
        .await
        .expect_err("step should reject response");

    assert!(matches!(
        error,
        crate::Error::Model(error) if error.core().kind == ErrorKind::InvalidOutput
            && error.core().message.contains("custom")
    ));
}

fn single_response_model(response: ModelResponse) -> Arc<dyn Model> {
    Arc::new(Unimock::new(
        ModelMock::stream
            .next_call(matching!(_))
            .answers_arc(stream_result(Ok(response))),
    ))
}

fn model_response(finish_reason: FinishReason, tool_calls: Vec<ToolCall>) -> ModelResponse {
    ModelResponse {
        attempts: Vec::new(),
        provider: "mock".to_owned(),
        model_id: "mock-model".to_owned(),
        catalog_model_id: None,
        thinking_level: None,
        parts: response_parts("done".to_owned(), tool_calls, Vec::new()),
        finish_reason,
        structured_output: None,
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
