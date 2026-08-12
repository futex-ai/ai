//! Model-call logging redaction regression tests.

use std::sync::Arc;

use ai_interface::{
    ConversationMessage, ConversationRole, FinishReason, Logger, LoggerMock,
    MiniMaxReasoningDetail, Model, ModelCallLogEntry, ModelCallLogResult, ModelError, ModelMock,
    ModelRequest, ModelResponse, ModelToolChoice, ModelUsage, ProviderConversationItem, ToolCall,
};
use serde_json::json;
use unimock::{MockFn, Unimock, matching};

use crate::{RunOutcome, Turn};

use super::support::{TypedEchoTool, runtime_with_logger, user_message};

#[derive(Debug, thiserror::Error)]
#[error("[ai_tool_calling/tests] model failed")]
struct FixtureModelError;

#[tokio::test]
async fn minimax_reasoning_is_replayed_but_redacted_from_success_logs() {
    let provider_context = minimax_context();
    let replay_context = provider_context.clone();
    let model: Arc<dyn Model> = Arc::new(Unimock::new((
        ModelMock::complete
            .next_call(matching!(_))
            .returns(Ok(tool_response(provider_context))),
        ModelMock::complete
            .next_call(matching!(_))
            .answers_arc(Arc::new(move |_, request: &ModelRequest| {
                let assistant = request
                    .messages
                    .iter()
                    .find(|message| message.role == ConversationRole::Assistant)
                    .expect("assistant replay message should be present");
                assert_eq!(assistant.provider_context, replay_context);
                Ok(stop_response())
            })),
    )));
    let logger: Arc<dyn Logger> = Arc::new(Unimock::new((
        LoggerMock::log_model_call.next_call(matching!(_)).answers(
            &|_, entry: &ModelCallLogEntry| {
                assert_log_entry_is_redacted(entry);
                Ok(())
            },
        ),
        LoggerMock::log_tool_activity
            .each_call(matching!(_))
            .answers(&|_, _| Ok(())),
        LoggerMock::log_tool_call
            .next_call(matching!(_))
            .returns(Ok(())),
        LoggerMock::log_model_call.next_call(matching!(_)).answers(
            &|_, entry: &ModelCallLogEntry| {
                assert_log_entry_is_redacted(entry);
                Ok(())
            },
        ),
        LoggerMock::log_turn_outcome
            .next_call(matching!(_))
            .returns(Ok(())),
    )));
    let tool = TypedEchoTool::succeeding();
    let runtime =
        runtime_with_logger(model, logger, vec![tool.tool()]).expect("runtime should build");

    let mut turn = runtime.send(user_message("start"), Some(4));
    let outcome = turn.run().await.expect("turn should complete");

    assert!(matches!(
        outcome,
        RunOutcome::Completed { steps_taken: 2, .. }
    ));
    assert!(runtime.conversation().iter().any(|message| {
        message.role == ConversationRole::Assistant
            && contains_minimax_context(&message.provider_context)
    }));
}

#[tokio::test]
async fn minimax_reasoning_is_redacted_from_failed_request_logs() {
    let provider_context = minimax_context();
    let replay_context = provider_context.clone();
    let model: Arc<dyn Model> = Arc::new(Unimock::new(
        ModelMock::complete
            .next_call(matching!(_))
            .answers_arc(Arc::new(move |_, request: &ModelRequest| {
                assert!(
                    request
                        .messages
                        .iter()
                        .any(|message| message.provider_context == replay_context)
                );
                Err(ModelError::internal(FixtureModelError))
            })),
    ));
    let logger: Arc<dyn Logger> = Arc::new(Unimock::new(
        LoggerMock::log_model_call.next_call(matching!(_)).answers(
            &|_, entry: &ModelCallLogEntry| {
                assert_request_is_redacted(&entry.request);
                assert_eq!(
                    entry.request.controls.generation.tool_choice,
                    Some(ModelToolChoice::RequiredOrAuto)
                );
                assert!(matches!(entry.result, ModelCallLogResult::Error { .. }));
                Ok(())
            },
        ),
    ));
    let runtime = runtime_with_logger(model, logger, Vec::new()).expect("runtime should build");
    runtime.replace_conversation(vec![ConversationMessage::assistant_with_provider_context(
        "calling tool",
        Vec::new(),
        provider_context,
    )]);

    let mut turn = runtime
        .resume(Some(1))
        .with_controls(ai_interface::ModelCallControls {
            generation: ai_interface::ModelGenerationControls {
                tool_choice: Some(ModelToolChoice::RequiredOrAuto),
                ..Default::default()
            },
            ..Default::default()
        });
    let error = turn.step().await.expect_err("model failure should surface");

    assert!(matches!(error, crate::Error::Model(_)));
}

fn minimax_context() -> Vec<ProviderConversationItem> {
    vec![ProviderConversationItem::MiniMaxAssistant {
        reasoning_content: Some("private chain of thought".to_owned()),
        reasoning_details: vec![MiniMaxReasoningDetail {
            kind: Some("reasoning.text".to_owned()),
            id: Some("reasoning-1".to_owned()),
            format: Some("MiniMax-response-v1".to_owned()),
            index: Some(0),
            text: Some("private detail".to_owned()),
        }],
    }]
}

fn tool_response(provider_context: Vec<ProviderConversationItem>) -> ModelResponse {
    ModelResponse {
        provider: "minimax".to_owned(),
        model_id: "MiniMax-M3".to_owned(),
        catalog_model_id: Some("MiniMax-M3".to_owned()),
        thinking_level: Some("medium".to_owned()),
        assistant_message: "calling tool".to_owned(),
        tool_calls: vec![ToolCall {
            id: "call-1".to_owned(),
            name: "echo".to_owned(),
            input: json!({ "message": "hello" }),
            operation_id: None,
        }],
        finish_reason: FinishReason::ToolCalls,
        structured_output: None,
        provider_context,
        usage: ModelUsage::default(),
    }
}

fn stop_response() -> ModelResponse {
    ModelResponse {
        provider: "minimax".to_owned(),
        model_id: "MiniMax-M3".to_owned(),
        catalog_model_id: Some("MiniMax-M3".to_owned()),
        thinking_level: Some("medium".to_owned()),
        assistant_message: "done".to_owned(),
        tool_calls: Vec::new(),
        finish_reason: FinishReason::Stop,
        structured_output: None,
        provider_context: Vec::new(),
        usage: ModelUsage::default(),
    }
}

fn assert_log_entry_is_redacted(entry: &ModelCallLogEntry) {
    assert_request_is_redacted(&entry.request);
    let ModelCallLogResult::Success { response } = &entry.result else {
        panic!("successful model call should have a response");
    };
    assert!(!contains_minimax_context(&response.provider_context));
}

fn assert_request_is_redacted(request: &ModelRequest) {
    assert!(
        request
            .messages
            .iter()
            .all(|message| !contains_minimax_context(&message.provider_context))
    );
}

fn contains_minimax_context(context: &[ProviderConversationItem]) -> bool {
    context
        .iter()
        .any(|item| matches!(item, ProviderConversationItem::MiniMaxAssistant { .. }))
}
