//! Reader advertisement follows the current store rather than past activity.

use std::sync::Arc;

use ai_interface::{
    ConversationMessage, FinishReason, ToolDefinition, ToolMock, ToolOutputId,
    test_support::model_response_stream,
};
use ai_tool_calling::{
    Agent, AgentEvent, InMemoryToolOutputStore, RunOutcome, ToolCallingRuntime, ToolOutputPolicy,
    ToolOutputStore, ToolOutputWriteRequest,
};
use serde_json::json;
use unimock::{MockFn, Unimock, matching};

use super::support::{call, collect, model, outcome, request, response};

fn reader_offered(request: &ai_interface::ModelRequest) -> bool {
    request
        .tools
        .iter()
        .any(|tool| tool.name == "tool_output_read")
}

#[tokio::test]
async fn reader_appears_only_after_storage_and_disappears_after_store_replacement() {
    let tool = Arc::new(Unimock::new((
        ToolMock::definitions
            .each_call(matching!())
            .returns(vec![ToolDefinition {
                name: "output".to_owned(),
                description: "Return output.".to_owned(),
                input_schema: json!({"type":"object"}),
                activity_verb: None,
            }]),
        ToolMock::call.stub(|each| {
            each.call(matching!("output", _))
                .answers(&|_, _, _| Ok(json!(1)))
                .once()
                .then()
                .answers(&|_, _, _| Ok(json!("longer than inline")))
                .once();
        }),
    )));
    let (model, requests) = model(vec![
        Ok(model_response_stream(response(
            "",
            vec![call("small", "output")],
            FinishReason::ToolCalls,
        ))),
        Ok(model_response_stream(response(
            "",
            vec![call("large", "output")],
            FinishReason::ToolCalls,
        ))),
        Ok(model_response_stream(response(
            "done",
            vec![],
            FinishReason::Stop,
        ))),
        Ok(model_response_stream(response(
            "again",
            vec![],
            FinishReason::Stop,
        ))),
    ]);
    let agent = ToolCallingRuntime::new(
        model,
        Arc::new(ai_interface::NoopLogger),
        vec![tool],
        Arc::new(InMemoryToolOutputStore::new()),
        ToolOutputPolicy::new(8, 8, 128, 256).unwrap(),
    )
    .unwrap();
    let mut conversation = vec![ConversationMessage::user("start")];
    let events = collect(agent.run(&mut conversation, request(None)).as_mut())
        .await
        .unwrap();
    assert!(matches!(
        outcome(&events),
        RunOutcome::Completed { steps_taken: 3, .. }
    ));
    assert_eq!(
        requests
            .lock()
            .unwrap()
            .iter()
            .map(reader_offered)
            .collect::<Vec<_>>(),
        [false, false, true]
    );
    agent.replace_output_store(Arc::new(InMemoryToolOutputStore::new()));
    collect(agent.run(&mut conversation, request(None)).as_mut())
        .await
        .unwrap();
    assert_eq!(
        requests
            .lock()
            .unwrap()
            .iter()
            .map(reader_offered)
            .collect::<Vec<_>>(),
        [false, false, true, false]
    );
}

#[tokio::test]
async fn a_store_filled_before_the_turn_offers_the_reader_immediately() {
    let store = Arc::new(InMemoryToolOutputStore::new());
    let policy = ToolOutputPolicy::new(8, 8, 128, 256).unwrap();
    let stored = store
        .write(ToolOutputWriteRequest {
            tool_name: "previous".to_owned(),
            content: "\"stored before turn\"".to_owned(),
            policy,
            first_window_length: 8,
        })
        .await
        .unwrap();
    let mut reader_call = call("read", "tool_output_read");
    reader_call.input = json!({"output_id":stored.output_id,"offset":0,"length":8});
    let (model, requests) = model(vec![
        Ok(model_response_stream(response(
            "",
            vec![reader_call],
            FinishReason::ToolCalls,
        ))),
        Ok(model_response_stream(response(
            "done",
            vec![],
            FinishReason::Stop,
        ))),
    ]);
    let agent = ToolCallingRuntime::new(
        model,
        Arc::new(ai_interface::NoopLogger),
        vec![],
        store,
        policy,
    )
    .unwrap();
    let mut conversation = vec![ConversationMessage::user("start")];
    let events = collect(agent.run(&mut conversation, request(None)).as_mut())
        .await
        .unwrap();
    assert!(reader_offered(&requests.lock().unwrap()[0]));
    let record = events
        .iter()
        .find_map(|event| match event {
            AgentEvent::ToolFinished {
                result: Ok(record), ..
            } => Some(record),
            _ => None,
        })
        .unwrap();
    assert_eq!(record.output_id, Some(stored.output_id));
    let ai_interface::ToolOutputEnvelope::Window(window) = &record.model_visible_output else {
        panic!("expected reader window")
    };
    assert_eq!(window.offset(), 0);
    assert!(matches!(
        outcome(&events),
        RunOutcome::Completed { steps_taken: 2, .. }
    ));
}

#[tokio::test]
async fn an_unoffered_reader_call_runs_and_its_error_does_not_end_the_turn() {
    let mut reader_call = call("read", "tool_output_read");
    let id: ToolOutputId =
        serde_json::from_value(json!("toolout_018f0000-0000-7000-8000-000000000001")).unwrap();
    reader_call.input = json!({"output_id":id});
    let (model, requests) = model(vec![
        Ok(model_response_stream(response(
            "",
            vec![reader_call],
            FinishReason::ToolCalls,
        ))),
        Ok(model_response_stream(response(
            "done",
            vec![],
            FinishReason::Stop,
        ))),
    ]);
    let store = Arc::new(InMemoryToolOutputStore::new());
    let agent = ToolCallingRuntime::new(
        model,
        Arc::new(ai_interface::NoopLogger),
        vec![],
        store.clone(),
        ToolOutputPolicy::default(),
    )
    .unwrap();
    let mut conversation = vec![ConversationMessage::user("start")];
    let events = collect(agent.run(&mut conversation, request(None)).as_mut())
        .await
        .unwrap();
    assert!(!reader_offered(&requests.lock().unwrap()[0]));
    assert!(events.iter().any(|event| matches!(event,AgentEvent::ToolFinished {call_id,result:Err(ai_interface::ToolError::Execution {..})} if call_id=="read")));
    assert!(
        conversation[2]
            .text()
            .contains("the original tool call itself succeeded")
    );
    assert!(matches!(
        outcome(&events),
        RunOutcome::Completed { steps_taken: 2, .. }
    ));
    assert_eq!(store.reserved_bytes(), 0);
}
