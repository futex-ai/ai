//! Observer failures cannot stop model or tool work.

use std::sync::{Arc, Mutex};

use ai_interface::{
    ConversationMessage, ConversationPart, FinishReason, LoggerError, LoggerMock,
    ModelCallLogEntry, ProviderId, ProviderState, ToolActivityLogEntry, ToolActivityPhase,
    ToolCallLogEntry, ToolCallLogResult, ToolDefinition, ToolMock, TurnOutcomeLogEntry,
    test_support::model_response_stream,
};
use ai_tool_calling::{
    Agent, AgentEvent, InMemoryToolOutputStore, RunOutcome, ToolCallingRuntime, ToolOutputPolicy,
};
use serde_json::json;
use unimock::{MockFn, Unimock, matching};

use super::support::{call, collect, model, outcome, request, response};

#[derive(Debug, thiserror::Error)]
#[error("private observer failure")]
struct ObserverFailure;

#[tokio::test]
async fn every_observer_error_is_ignored_and_each_payload_is_a_log_copy() {
    let calls = Arc::new(Mutex::new(Vec::new()));
    let payloads = Arc::new(Mutex::new(Vec::new()));
    let tool_logs = Arc::new(Mutex::new(Vec::new()));
    let logger = Unimock::new((
        LoggerMock::log_model_call
            .each_call(matching!(_))
            .answers_arc({
                let calls = calls.clone();
                let payloads = payloads.clone();
                Arc::new(move |_, entry: &ModelCallLogEntry| {
                    calls.lock().unwrap().push("model");
                    payloads.lock().unwrap().push(format!("{entry:?}"));
                    Err(LoggerError::write(ObserverFailure))
                })
            }),
        LoggerMock::log_tool_call
            .each_call(matching!(_))
            .answers_arc({
                let calls = calls.clone();
                let tool_logs = tool_logs.clone();
                Arc::new(move |_, entry: &ToolCallLogEntry| {
                    calls.lock().unwrap().push("tool");
                    tool_logs.lock().unwrap().push(entry.clone());
                    Err(LoggerError::write(ObserverFailure))
                })
            }),
        LoggerMock::log_tool_activity
            .each_call(matching!(_))
            .answers_arc({
                let calls = calls.clone();
                Arc::new(move |_, entry: &ToolActivityLogEntry| {
                    calls.lock().unwrap().push(match entry.phase {
                        ToolActivityPhase::Started => "start",
                        ToolActivityPhase::Completed => "finish",
                    });
                    assert_eq!(entry.activity_verb.as_deref(), Some("Working"));
                    Err(LoggerError::write(ObserverFailure))
                })
            }),
        LoggerMock::log_turn_outcome
            .each_call(matching!(_))
            .answers_arc({
                let calls = calls.clone();
                Arc::new(move |_, _: &TurnOutcomeLogEntry| {
                    calls.lock().unwrap().push("outcome");
                    Err(LoggerError::write(ObserverFailure))
                })
            }),
    ));
    let tool = Arc::new(Unimock::new((
        ToolMock::definitions
            .each_call(matching!())
            .returns(vec![ToolDefinition {
                name: "work".to_owned(),
                description: "Do work.".to_owned(),
                input_schema: json!({"type":"object"}),
                activity_verb: Some("Working".to_owned()),
            }]),
        ToolMock::group_for_tool
            .each_call(matching!("work"))
            .returns(Some("work-group")),
        ToolMock::call
            .some_call(matching!("work", _))
            .returns(Ok(json!("aaaaaaaaaaaaaaaa-private-tool-output"))),
    )));
    let state = ProviderState {
        provider: ProviderId::new("test"),
        model: "test-model".to_owned(),
        version: 1,
        payload: json!({"signature":"private-provider-state"})
            .as_object()
            .unwrap()
            .clone(),
    };
    let mut first = response(
        "working",
        vec![call("work-1", "work")],
        FinishReason::ToolCalls,
    );
    first.parts.insert(
        0,
        ConversationPart::reasoning("private-reasoning").with_provider_state(state),
    );
    let (model, _) = model(vec![
        Ok(model_response_stream(first)),
        Ok(model_response_stream(response(
            "done",
            vec![],
            FinishReason::Stop,
        ))),
    ]);
    let agent = ToolCallingRuntime::new(
        model,
        Arc::new(logger),
        vec![tool],
        Arc::new(InMemoryToolOutputStore::new()),
        ToolOutputPolicy::new(8, 8, 128, 256).unwrap(),
    )
    .unwrap();
    let mut conversation = vec![ConversationMessage::user("start")];
    let result = collect(agent.run(&mut conversation, request(None)).as_mut()).await;
    assert!(
        result.is_ok(),
        "observer errors must not fail the turn: {result:?}"
    );
    let events = result.unwrap();
    assert!(matches!(
        outcome(&events),
        RunOutcome::Completed { steps_taken: 2, .. }
    ));
    assert_eq!(
        *calls.lock().unwrap(),
        ["model", "start", "tool", "finish", "model", "outcome"]
    );
    let payloads = payloads.lock().unwrap().join("\n");
    for secret in [
        "private-provider-state",
        "private-reasoning",
        "private-tool-output",
    ] {
        assert!(!payloads.contains(secret), "observer leaked {secret}");
    }
    let logs = tool_logs.lock().unwrap();
    assert_eq!(logs[0].tool_group.as_deref(), Some("work-group"));
    assert!(matches!(logs[0].result, ToolCallLogResult::Success { .. }));
    assert!(!format!("{:?}", logs[0]).contains("private-tool-output"));
    let record = events
        .iter()
        .find_map(|event| match event {
            AgentEvent::ToolFinished {
                result: Ok(record), ..
            } => Some(record),
            _ => None,
        })
        .unwrap();
    assert_eq!(
        record.raw_output,
        json!("aaaaaaaaaaaaaaaa-private-tool-output")
    );
    assert_eq!(
        conversation[1].parts[0].provider_state().unwrap().payload["signature"],
        "private-provider-state"
    );
}
