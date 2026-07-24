use std::sync::Arc;

use ai_interface::{
    ConversationRole, DynModel, DynTool, FinishReason, ModelMock, ModelRequest, ModelResponse,
    ModelUsage, ToolCall, ToolDefinition, ToolMock, ToolOutputEnvelope,
};
use serde_json::{Value, json};
use unimock::{MockFn, Unimock, matching};

use crate::{InMemoryToolOutputStore, RunOutcome, ToolOutputPolicy, Turn};

use super::super::support::{runtime_with_store_and_policy, user_message};

#[tokio::test]
async fn multi_window_output_can_be_followed_to_completion() {
    let policy = ToolOutputPolicy::new(5, 5, 100, 100).unwrap();
    let model: DynModel = Arc::new(Unimock::new((
        ModelMock::complete
            .next_call(matching!(_))
            .returns(Ok(tool_call_response(tool_call(
                "call-big",
                "big_output",
                json!({}),
            )))),
        ModelMock::complete
            .next_call(matching!(_))
            .answers(&|_, request: &ModelRequest| {
                let window = last_tool_window(request);
                assert_eq!(window.content(), "\"abcd");
                assert_eq!(window.next_offset(), Some(5));
                assert_first_window_does_not_expose_full_raw_output(request);
                Ok(tool_call_response(tool_call(
                    "call-read-1",
                    "tool_output_read",
                    json!({
                        "output_id": window.output_id().unwrap(),
                        "offset": window.next_offset().unwrap()
                    }),
                )))
            }),
        ModelMock::complete
            .next_call(matching!(_))
            .answers(&|_, request: &ModelRequest| {
                let window = last_tool_window(request);
                assert_eq!(window.content(), "efghi");
                assert_eq!(window.next_offset(), Some(10));
                Ok(tool_call_response(tool_call(
                    "call-read-2",
                    "tool_output_read",
                    json!({
                        "output_id": window.output_id().unwrap(),
                        "offset": window.next_offset().unwrap()
                    }),
                )))
            }),
        ModelMock::complete
            .next_call(matching!(_))
            .answers(&|_, request: &ModelRequest| {
                let window = last_tool_window(request);
                assert_eq!(window.content(), "j\"");
                assert!(!window.truncated());
                Ok(stop_response())
            }),
    )));
    let runtime = runtime_with_store_and_policy(
        model,
        vec![output_tool("big_output", json!("abcdefghij"))],
        Arc::new(InMemoryToolOutputStore::new()),
        policy,
    )
    .expect("runtime should build");

    let mut turn = runtime.send(user_message("start"), Some(5));
    let outcome = turn.run().await.expect("pagination run should complete");

    assert!(matches!(
        outcome,
        RunOutcome::Completed {
            assistant_message,
            steps_taken: 4,
        } if assistant_message == "done"
    ));
    assert_eq!(turn.successful_tool_calls().len(), 3);
    assert_eq!(
        turn.successful_tool_calls()[1].raw_output["type"],
        json!("tool_output_window")
    );
    assert_eq!(
        turn.successful_tool_calls()[1]
            .model_visible_output
            .output_id(),
        turn.successful_tool_calls()[1].output_id.as_ref()
    );
}

fn assert_first_window_does_not_expose_full_raw_output(request: &ModelRequest) {
    let tool_message = request
        .messages
        .iter()
        .find(|message| message.tool_call_id.as_deref() == Some("call-big"))
        .expect("first tool message should be retained");
    assert!(
        tool_message
            .content
            .contains("\"type\":\"tool_output_window\"")
    );
    assert!(!tool_message.content.contains("abcdefghij"));
}

fn last_tool_window(request: &ModelRequest) -> ai_interface::ToolOutputWindowEnvelope {
    let message = request
        .messages
        .iter()
        .rev()
        .find(|message| message.role == ConversationRole::Tool)
        .expect("tool message should be retained");
    let envelope: ToolOutputEnvelope =
        serde_json::from_str(&message.content).expect("tool message should be an envelope");
    match envelope {
        ToolOutputEnvelope::Window(window) => window,
        ToolOutputEnvelope::Inline(_) => panic!("expected window envelope"),
    }
}

fn output_tool(name: &str, output: Value) -> DynTool {
    Arc::new(Unimock::new((
        ToolMock::definitions
            .next_call(matching!())
            .returns(vec![ToolDefinition {
                name: name.to_owned(),
                description: "Return a static value.".to_owned(),
                input_schema: json!({ "type": "object" }),
                activity_verb: None,
            }]),
        ToolMock::call
            .each_call(matching!(_, _))
            .answers_arc(Arc::new(move |_, _, _| Ok(output.clone()))),
    )))
}

fn tool_call(id: &str, name: &str, input: Value) -> ToolCall {
    ToolCall {
        id: id.to_owned(),
        name: name.to_owned(),
        input,
        operation_id: None,
    }
}

fn tool_call_response(call: ToolCall) -> ModelResponse {
    ModelResponse {
        provider: "mock".to_owned(),
        model_id: "mock-model".to_owned(),
        catalog_model_id: None,
        thinking_level: None,
        assistant_message: "calling tool".to_owned(),
        tool_calls: vec![call],
        finish_reason: FinishReason::ToolCalls,
        structured_output: None,
        provider_context: Vec::new(),
        usage: ModelUsage::default(),
    }
}

fn stop_response() -> ModelResponse {
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
