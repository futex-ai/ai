//! Qwen model-call logging redaction regression tests.

use std::sync::Arc;

use ai_interface::test_support::model_response_stream;
use ai_interface::{
    ConversationMessage, ConversationPart, ConversationRole, Effort, FinishReason, Logger,
    LoggerMock, Model, ModelCallLogEntry, ModelCallLogResult, ModelError, ModelMock, ModelRequest,
    ModelResponse, ModelUsage, ToolCall,
};
use serde_json::json;
use unimock::{MockFn, Unimock, matching};

use crate::tests::support::{provider_reasoning, replay_parts, response_parts, stream_result};
use crate::{RunOutcome, Turn};

use super::support::{TypedEchoTool, runtime_with_logger, user_message};

#[derive(Debug, thiserror::Error)]
#[error("[ai_tool_calling/tests] model failed")]
struct FixtureModelError;

#[tokio::test]
async fn qwen_reasoning_is_replayed_but_redacted_from_success_logs() {
    let provider_context = qwen_context();
    let replay_context = provider_context.clone();
    let model: Arc<dyn Model> = Arc::new(Unimock::new((
        ModelMock::stream
            .next_call(matching!(_))
            .answers_arc(stream_result(Ok(tool_response(provider_context)))),
        ModelMock::stream
            .next_call(matching!(_))
            .answers_arc(Arc::new(move |_, request: &ModelRequest| {
                {
                    let assistant = request
                        .messages
                        .iter()
                        .find(|message| message.role == ConversationRole::Assistant)
                        .expect("assistant replay message should be present");
                    assert_eq!(replay_parts(&assistant.parts), replay_context);
                    Ok(stop_response())
                }
                .map(model_response_stream)
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
        message.role == ConversationRole::Assistant && contains_qwen_context(&message.parts)
    }));
}

#[tokio::test]
async fn qwen_reasoning_is_redacted_from_failed_request_logs() {
    let provider_context = qwen_context();
    let replay_context = provider_context.clone();
    let model: Arc<dyn Model> = Arc::new(Unimock::new(
        ModelMock::stream
            .next_call(matching!(_))
            .answers_arc(Arc::new(move |_, request: &ModelRequest| {
                {
                    assert!(
                        request
                            .messages
                            .iter()
                            .any(|message| replay_parts(&message.parts) == replay_context)
                    );
                    Err(ModelError::internal(
                        "qwen",
                        "qwen3.7-plus",
                        FixtureModelError,
                    ))
                }
                .map(model_response_stream)
            })),
    ));
    let logger: Arc<dyn Logger> = Arc::new(Unimock::new(
        LoggerMock::log_model_call.next_call(matching!(_)).answers(
            &|_, entry: &ModelCallLogEntry| {
                assert_request_is_redacted(&entry.request);
                assert!(matches!(entry.result, ModelCallLogResult::Error { .. }));
                Ok(())
            },
        ),
    ));
    let runtime = runtime_with_logger(model, logger, Vec::new()).expect("runtime should build");
    runtime.replace_conversation(vec![ConversationMessage::new(
        ConversationRole::Assistant,
        response_parts("calling tool", Vec::new(), provider_context),
    )]);

    let mut turn = runtime.resume(Some(1));
    let error = turn.step().await.expect_err("model failure should surface");

    assert!(matches!(error, crate::Error::Model(_)));
}

fn qwen_context() -> Vec<ConversationPart> {
    vec![provider_reasoning(
        "qwen",
        "qwen3.7-plus",
        "private Qwen reasoning",
        json!({
            "content": "calling tool", "reasoning_content": "private Qwen reasoning",
            "tool_calls": [{"id": "call-1", "name": "echo", "arguments": "{\"message\":\"hello\"}"}]
        }),
    )]
}

fn tool_response(provider_context: Vec<ConversationPart>) -> ModelResponse {
    ModelResponse {
        attempts: Vec::new(),
        provider: "qwen".to_owned(),
        model_id: "qwen3.7-plus".to_owned(),
        catalog_model_id: Some("qwen3.7-plus".to_owned()),
        thinking_level: Some(Effort::High),
        parts: response_parts(
            "calling tool".to_owned(),
            vec![ToolCall {
                id: "call-1".to_owned(),
                name: "echo".to_owned(),
                input: json!({ "message": "hello" }),
                operation_id: None,
            }],
            provider_context,
        ),
        finish_reason: FinishReason::ToolCalls,
        structured_output: None,
        usage: ModelUsage::default(),
    }
}

fn stop_response() -> ModelResponse {
    ModelResponse {
        attempts: Vec::new(),
        provider: "qwen".to_owned(),
        model_id: "qwen3.7-plus".to_owned(),
        catalog_model_id: Some("qwen3.7-plus".to_owned()),
        thinking_level: Some(Effort::High),
        parts: response_parts("done".to_owned(), Vec::new(), Vec::new()),
        finish_reason: FinishReason::Stop,
        structured_output: None,
        usage: ModelUsage::default(),
    }
}

fn assert_log_entry_is_redacted(entry: &ModelCallLogEntry) {
    assert_request_is_redacted(&entry.request);
    let ModelCallLogResult::Success { response } = &entry.result else {
        panic!("successful model call should have a response");
    };
    assert!(!contains_qwen_context(&response.parts));
}

fn assert_request_is_redacted(request: &ModelRequest) {
    assert!(
        request
            .messages
            .iter()
            .all(|message| !contains_qwen_context(&message.parts))
    );
}

fn contains_qwen_context(parts: &[ConversationPart]) -> bool {
    parts.iter().any(|part| {
        part.provider_state()
            .is_some_and(|state| state.provider.as_str() == "qwen")
    })
}
