//! Signed replay payloads stay in model history and out of logger copies.

use std::sync::Arc;

use ai_interface::test_support::model_response_stream;
use ai_interface::{
    ConversationMessage, ConversationPart, ConversationRole, ErrorKind, FinishReason, LoggerMock,
    ModelCallLogEntry, ModelCallLogResult, ModelError, ModelMock, ModelRequest, ModelResponse,
    ModelUsage, ToolCall,
};
use serde_json::json;
use unimock::{MockFn, Unimock, matching};

use crate::tests::event_support::{collect_events, request, run_outcome};
use crate::tests::support::{response_parts, stream_result};
use crate::{Agent, RunOutcome};

use super::support::{TypedEchoTool, provider_reasoning, runtime_with_logger, user_message};

#[tokio::test]
async fn signed_replay_context_survives_turns_but_not_success_logs() {
    let context = signed_context();
    let expected = response(context.clone()).parts;
    let model = Arc::new(Unimock::new((
        ModelMock::stream
            .next_call(matching!(_))
            .answers_arc(stream_result(Ok(response(context)))),
        ModelMock::stream
            .next_call(matching!(_))
            .answers_arc(Arc::new(move |_, request: &ModelRequest| {
                {
                    let assistant = request
                        .messages
                        .iter()
                        .find(|m| m.role == ConversationRole::Assistant)
                        .unwrap();
                    assert_eq!(assistant.parts, expected);
                    let mut response = response(signed_context());
                    response.finish_reason = FinishReason::Stop;
                    response
                        .parts
                        .retain(|part| matches!(part, ConversationPart::Reasoning { .. }));
                    Ok(response)
                }
                .map(model_response_stream)
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
                        .all(|m| private_parts(&m.parts).is_empty())
                );
                let ModelCallLogResult::Success { response } = &entry.result else {
                    panic!("expected success")
                };
                assert_eq!(
                    private_parts(&response.parts),
                    Vec::<ConversationPart>::new()
                );
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
    let mut conversation = vec![user_message("start")];
    let events = collect_events(runtime.run(&mut conversation, request(Some(4))))
        .await
        .unwrap();
    assert_eq!(
        run_outcome(&events),
        RunOutcome::Completed {
            assistant_message: "visible".to_owned(),
            steps_taken: 2,
            structured_output: None,
        }
    );
    let assistant = conversation
        .iter()
        .find(|m| m.role == ConversationRole::Assistant)
        .unwrap();
    assert_eq!(assistant.parts, response(signed_context()).parts);
    assert_eq!(conversation.last().unwrap().parts, signed_context());
}

#[tokio::test]
async fn signed_replay_context_is_removed_from_failed_request_logs() {
    let context = signed_context();
    let expected = response_parts("visible", Vec::new(), context.clone());
    let model = Arc::new(Unimock::new(
        ModelMock::stream
            .some_call(matching!(_))
            .answers_arc(Arc::new(move |_, request: &ModelRequest| {
                {
                    assert_eq!(request.messages[0].parts, expected);
                    Err(ModelError::invalid_output("mock", "mock", "failed"))
                }
                .map(model_response_stream)
            })),
    ));
    let logger = Arc::new(Unimock::new(
        LoggerMock::log_model_call.some_call(matching!(_)).answers(
            &|_, entry: &ModelCallLogEntry| {
                assert_eq!(
                    private_parts(&entry.request.messages[0].parts),
                    Vec::<ConversationPart>::new()
                );
                assert!(matches!(entry.result, ModelCallLogResult::Error { .. }));
                Ok(())
            },
        ),
    ));
    let runtime = runtime_with_logger(model, logger, Vec::new()).unwrap();
    let mut conversation = vec![ConversationMessage::new(
        ConversationRole::Assistant,
        response_parts("visible", Vec::new(), context),
    )];
    assert!(matches!(
        collect_events(runtime.run(&mut conversation, request(Some(1)))).await,
        Err(crate::Error::Model(error)) if error.core().kind == ErrorKind::InvalidOutput
    ));
}

fn signed_context() -> Vec<ConversationPart> {
    vec![
        provider_reasoning(
            "anthropic",
            "claude",
            "private",
            json!({"blocks": [
                {"type": "thinking", "thinking": "private", "signature": "private-signature"},
                {"type": "redacted_thinking", "data": "private-data"}
            ]}),
        ),
        provider_reasoning(
            "google",
            "gemini",
            "private",
            json!({"parts": [
                {"text": "private", "thought": true, "thoughtSignature": "private-signature"}
            ]}),
        ),
        provider_reasoning(
            "openai",
            "gpt",
            "private summary",
            json!({
                "id": "reasoning-id", "encrypted_content": "private-encrypted-reasoning"
            }),
        ),
        ConversationPart::reasoning("private reasoning without state"),
    ]
}

fn private_parts(parts: &[ConversationPart]) -> Vec<ConversationPart> {
    parts
        .iter()
        .filter(|part| {
            part.provider_state().is_some()
                || matches!(part, ConversationPart::Reasoning { text, .. } if !text.is_empty())
        })
        .cloned()
        .collect()
}

fn response(mut parts: Vec<ConversationPart>) -> ModelResponse {
    let state = provider_reasoning(
        "future-provider",
        "future-model",
        "",
        json!({
            "opaque": "private-call-state"
        }),
    )
    .provider_state()
    .unwrap()
    .clone();
    let call = ConversationPart::from_tool_call(ToolCall {
        id: "call-1".to_owned(),
        name: "echo".to_owned(),
        input: json!({"message": "hello"}),
        operation_id: None,
    })
    .with_provider_state(state);
    let text_state = provider_reasoning(
        "openai",
        "gpt",
        "",
        json!({
            "phase": "commentary"
        }),
    )
    .provider_state()
    .unwrap()
    .clone();
    if parts.is_empty() {
        parts.push(call);
    } else {
        parts.insert(1, call);
    }
    parts.push(ConversationPart::text("visible").with_provider_state(text_state));
    ModelResponse {
        attempts: Vec::new(),
        provider: "mock".to_owned(),
        model_id: "mock".to_owned(),
        catalog_model_id: None,
        thinking_level: None,
        parts,
        finish_reason: FinishReason::ToolCalls,
        structured_output: None,
        usage: ModelUsage::default(),
    }
}
