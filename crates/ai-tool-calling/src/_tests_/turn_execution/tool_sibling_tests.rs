//! Recoverable failures preserve every sibling tool result.

use std::sync::Arc;

use ai_interface::test_support::model_response_stream;
use ai_interface::{
    FinishReason, Model, ModelMock, ModelRequest, ModelResponse, ModelUsage, ToolCall, ToolError,
};
use serde_json::json;
use unimock::{MockFn, Unimock, matching};

use crate::Agent;
use crate::RunOutcome;
use crate::tests::event_support::{collect_events, request, run_outcome, tool_errors};
use crate::tests::support::{
    TypedEchoTool, response_parts, runtime, stream_result, tool_result_id, user_message,
};

#[tokio::test]
async fn failed_tool_calls_still_append_messages_for_later_calls_in_the_same_batch() {
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
            }))),
        ModelMock::stream
            .next_call(matching!(_))
            .answers(&|_, request: &ModelRequest| {
                {
                    let tool_messages = request
                        .messages
                        .iter()
                        .filter(|message| message.role == ai_interface::ConversationRole::Tool)
                        .cloned()
                        .collect::<Vec<_>>();
                    assert_eq!(tool_messages.len(), 2);
                    assert_eq!(tool_result_id(&tool_messages[0]), Some("call-1"),);
                    assert_eq!(tool_result_id(&tool_messages[1]), Some("call-2"),);
                    assert!(tool_messages[0].text().contains("\"ok\":false"));
                    assert!(tool_messages[1].text().contains("\"echo\":\"second\""));
                    Ok(ModelResponse {
                        attempts: Vec::new(),
                        provider: "mock".to_owned(),
                        model_id: "mock-model".to_owned(),
                        catalog_model_id: None,
                        thinking_level: None,
                        parts: response_parts("recovered".to_owned(), Vec::new(), Vec::new()),
                        finish_reason: FinishReason::Stop,
                        structured_output: None,
                        usage: ModelUsage::default(),
                    })
                }
                .map(model_response_stream)
            }),
    )));
    let tool = TypedEchoTool::fail_once();
    let runtime = runtime(model, vec![tool.tool()]).expect("runtime should build");

    let mut conversation = vec![user_message("start")];
    let events = collect_events(runtime.run(&mut conversation, request(Some(4))))
        .await
        .expect("tool error should not stop the turn");
    assert!(matches!(
        tool_errors(&events).as_slice(),
        [ToolError::Execution { .. }]
    ));
    assert_eq!(tool.parse_count(), 2);

    let outcome = run_outcome(&events);
    assert!(matches!(
        outcome,
        RunOutcome::Completed {
            assistant_message,
            steps_taken: 2,
            ..
        } if assistant_message == "recovered"
    ));
}
