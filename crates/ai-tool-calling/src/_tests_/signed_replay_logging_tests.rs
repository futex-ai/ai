//! Signed replay payloads stay in model history and out of logger copies.

use std::sync::Arc;

use ai_interface::{
    ConversationMessage, ConversationRole, FinishReason, LoggerMock, ModelCallLogEntry,
    ModelCallLogResult, ModelError, ModelMock, ModelRequest, ModelResponse, ModelUsage,
    ProviderConversationItem, ToolCall,
};
use serde_json::json;
use unimock::{MockFn, Unimock, matching};

use crate::{RunOutcome, Turn};

use super::support::{TypedEchoTool, runtime_with_logger, user_message};

#[tokio::test]
async fn signed_replay_context_survives_turns_but_not_success_logs() {
    let context = signed_context();
    let expected = context.clone();
    let model = Arc::new(Unimock::new((
        ModelMock::complete
            .next_call(matching!(_))
            .returns(Ok(response(context))),
        ModelMock::complete
            .next_call(matching!(_))
            .answers_arc(Arc::new(move |_, request: &ModelRequest| {
                let assistant = request
                    .messages
                    .iter()
                    .find(|m| m.role == ConversationRole::Assistant)
                    .unwrap();
                assert_eq!(assistant.provider_context, expected);
                let mut response = response(Vec::new());
                response.finish_reason = FinishReason::Stop;
                response.tool_calls.clear();
                Ok(response)
            })),
    )));
    let logger = Arc::new(Unimock::new((
        LoggerMock::log_model_call.each_call(matching!(_)).answers(
            &|_, entry: &ModelCallLogEntry| {
                assert!(
                    entry
                        .request
                        .messages
                        .iter()
                        .all(|m| m.provider_context.is_empty())
                );
                let ModelCallLogResult::Success { response } = &entry.result else {
                    panic!("expected success")
                };
                assert_eq!(response.provider_context, Vec::new());
                Ok(())
            },
        ),
        LoggerMock::log_tool_activity
            .each_call(matching!(_))
            .answers(&|_, _| Ok(())),
        LoggerMock::log_tool_call
            .some_call(matching!(_))
            .returns(Ok(())),
        LoggerMock::log_turn_outcome
            .some_call(matching!(_))
            .returns(Ok(())),
    )));
    let runtime =
        runtime_with_logger(model, logger, vec![TypedEchoTool::succeeding().tool()]).unwrap();
    let mut turn = runtime.send(user_message("start"), Some(4));
    assert_eq!(
        turn.run().await.unwrap(),
        RunOutcome::Completed {
            assistant_message: "visible".to_owned(),
            steps_taken: 2
        }
    );
    let conversation = runtime.conversation();
    let assistant = conversation
        .iter()
        .find(|m| m.role == ConversationRole::Assistant)
        .unwrap();
    assert_eq!(assistant.provider_context, signed_context());
}

#[tokio::test]
async fn signed_replay_context_is_removed_from_failed_request_logs() {
    let context = signed_context();
    let expected = context.clone();
    let model = Arc::new(Unimock::new(
        ModelMock::complete
            .some_call(matching!(_))
            .answers_arc(Arc::new(move |_, request: &ModelRequest| {
                assert_eq!(request.messages[0].provider_context, expected);
                Err(ModelError::provider("mock", "mock", "failed"))
            })),
    ));
    let logger = Arc::new(Unimock::new(
        LoggerMock::log_model_call.some_call(matching!(_)).answers(
            &|_, entry: &ModelCallLogEntry| {
                assert_eq!(entry.request.messages[0].provider_context, Vec::new());
                assert!(matches!(entry.result, ModelCallLogResult::Error { .. }));
                Ok(())
            },
        ),
    ));
    let runtime = runtime_with_logger(model, logger, Vec::new()).unwrap();
    runtime.replace_conversation(vec![ConversationMessage::assistant_with_provider_context(
        "visible",
        Vec::new(),
        context,
    )]);
    assert!(matches!(
        runtime.resume(Some(1)).step().await,
        Err(crate::Error::Model(ModelError::Provider { .. }))
    ));
}

fn signed_context() -> Vec<ProviderConversationItem> {
    serde_json::from_value(json!([
        {"type": "anthropic_assistant_message", "blocks": [
            {"type": "thinking", "thinking": "private", "signature": "private-signature"},
            {"type": "redacted_thinking", "data": "private-data"}
        ]},
        {"type": "google_assistant_message", "parts": [
            {"text": "private", "thought": true, "thoughtSignature": "private-signature"}
        ]}
    ]))
    .unwrap()
}

fn response(provider_context: Vec<ProviderConversationItem>) -> ModelResponse {
    ModelResponse {
        provider: "mock".to_owned(),
        model_id: "mock".to_owned(),
        catalog_model_id: None,
        thinking_level: None,
        assistant_message: "visible".to_owned(),
        tool_calls: vec![ToolCall {
            id: "call-1".to_owned(),
            name: "echo".to_owned(),
            input: json!({"message": "hello"}),
            operation_id: None,
        }],
        finish_reason: FinishReason::ToolCalls,
        structured_output: None,
        provider_context,
        usage: ModelUsage::default(),
    }
}
