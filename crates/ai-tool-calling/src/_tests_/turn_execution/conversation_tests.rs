//! Runtime tests through the model stream contract.

use std::sync::Arc;

use ai_interface::test_support::model_response_stream;
use ai_interface::{
    ConversationRole, Effort, FinishReason, Logger, LoggerMock, Model, ModelMock, ModelRequest,
    ModelResponse, ModelUsage, ToolActivityLogEntry, ToolActivityPhase, ToolCall, ToolCallLogEntry,
    ToolCallLogResult,
};
use serde_json::json;
use unimock::{MockFn, Unimock, matching};

use crate::tests::support::{
    provider_reasoning, replay_parts, response_parts, stream_result, tool_result_name,
};
use crate::{RunOutcome, StepOutcome, Turn};

use super::super::support::{TypedEchoTool, runtime, runtime_with_logger, user_message};

#[tokio::test]
async fn retained_conversation_is_used_across_future_sends() {
    let model: Arc<dyn Model> = Arc::new(Unimock::new((
        ModelMock::stream
            .next_call(matching!(_))
            .answers_arc(stream_result(Ok(ModelResponse {
                attempts: Vec::new(),
                provider: "mock".to_owned(),
                model_id: "mock-model".to_owned(),
                catalog_model_id: None,
                thinking_level: None,
                parts: response_parts("first response".to_owned(), Vec::new(), Vec::new()),
                finish_reason: FinishReason::Stop,
                structured_output: None,
                usage: ModelUsage::default(),
            }))),
        ModelMock::stream
            .next_call(matching!(_))
            .answers(&|_, request: &ModelRequest| {
                {
                    assert!(request.messages.iter().any(|message| {
                        message.role == ConversationRole::Assistant
                            && message.text() == "first response"
                    }));
                    Ok(ModelResponse {
                        attempts: Vec::new(),
                        provider: "mock".to_owned(),
                        model_id: "mock-model".to_owned(),
                        catalog_model_id: None,
                        thinking_level: None,
                        parts: response_parts("second response".to_owned(), Vec::new(), Vec::new()),
                        finish_reason: FinishReason::Stop,
                        structured_output: None,
                        usage: ModelUsage::default(),
                    })
                }
                .map(model_response_stream)
            }),
    )));
    let runtime = runtime(model, Vec::new()).expect("runtime should build");

    let mut first_turn = runtime.send(user_message("first message"), Some(4));
    first_turn.run().await.expect("first run should succeed");

    let mut second_turn = runtime.send(user_message("second message"), Some(4));
    let outcome = second_turn.run().await.expect("second run should succeed");
    assert!(matches!(
        outcome,
        RunOutcome::Completed {
            assistant_message,
            steps_taken: 1,
        } if assistant_message == "second response"
    ));
}

#[tokio::test]
async fn provider_context_is_retained_with_assistant_tool_calls() {
    let provider_context = vec![provider_reasoning(
        "openai",
        "gpt-5.5",
        "Need the tool.",
        json!({
            "id": "rs_1", "summary": [{"type": "summary_text", "text": "Need the tool."}],
            "encrypted_content": "encrypted-reasoning"
        }),
    )];
    let model: Arc<dyn Model> = Arc::new(Unimock::new(
        ModelMock::stream
            .next_call(matching!(_))
            .answers_arc(stream_result(Ok(ModelResponse {
                attempts: Vec::new(),
                provider: "openai".to_owned(),
                model_id: "gpt-5.5".to_owned(),
                catalog_model_id: None,
                thinking_level: Some(Effort::ExtraHigh),
                parts: response_parts(
                    "calling tool".to_owned(),
                    vec![ToolCall {
                        id: "call-1".to_owned(),
                        name: "echo".to_owned(),
                        input: json!({ "message": "hello" }),
                        operation_id: None,
                    }],
                    provider_context.clone(),
                ),
                finish_reason: FinishReason::ToolCalls,
                structured_output: None,
                usage: ModelUsage::default(),
            }))),
    ));
    let tool = TypedEchoTool::succeeding();
    let runtime = runtime(model, vec![tool.tool()]).expect("runtime should build");
    let mut turn = runtime.send(user_message("start"), Some(1));

    turn.step().await.expect("step should succeed");

    let conversation = runtime.conversation();
    let assistant = conversation
        .iter()
        .find(|message| message.role == ConversationRole::Assistant)
        .expect("assistant message should be retained");
    assert_eq!(replay_parts(&assistant.parts), provider_context);
}

#[tokio::test]
async fn kimi_reasoning_is_retained_without_becoming_visible_output() {
    let reasoning = "private Kimi reasoning";
    let provider_context = vec![provider_reasoning(
        "kimi",
        "kimi-k3",
        reasoning,
        json!({
            "content": "calling echo", "reasoning_content": reasoning,
            "tool_calls": [{"id": "call-1", "name": "echo", "arguments": "{\"message\":\"hello\"}"}]
        }),
    )];
    let model: Arc<dyn Model> = Arc::new(Unimock::new(
        ModelMock::stream
            .next_call(matching!(_))
            .answers_arc(stream_result(Ok(ModelResponse {
                attempts: Vec::new(),
                provider: "kimi".to_owned(),
                model_id: "kimi-k3".to_owned(),
                catalog_model_id: Some("kimi-k3".to_owned()),
                thinking_level: Some(Effort::Max),
                parts: response_parts(
                    "calling echo".to_owned(),
                    vec![ToolCall {
                        id: "call-1".to_owned(),
                        name: "echo".to_owned(),
                        input: json!({"message": "hello"}),
                        operation_id: None,
                    }],
                    provider_context.clone(),
                ),
                finish_reason: FinishReason::ToolCalls,
                structured_output: None,
                usage: ModelUsage::default(),
            }))),
    ));
    let tool = TypedEchoTool::succeeding();
    let runtime = runtime(model, vec![tool.tool()]).expect("runtime should build");
    let mut turn = runtime.send(user_message("start"), Some(1));

    turn.step().await.expect("step should succeed");

    let conversation = runtime.conversation();
    let assistant = conversation
        .iter()
        .find(|message| message.role == ConversationRole::Assistant)
        .expect("assistant message should be retained");
    let tool_result = conversation
        .iter()
        .find(|message| message.role == ConversationRole::Tool)
        .expect("tool result should be retained");
    assert_eq!(assistant.text(), "calling echo");
    assert_eq!(replay_parts(&assistant.parts), provider_context);
    assert!(!assistant.text().contains(reasoning));
    assert!(!tool_result.text().contains(reasoning));
}

#[tokio::test]
async fn step_and_run_handle_tool_rounds_and_logger_callbacks() {
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
                    "calling tool".to_owned(),
                    vec![ToolCall {
                        id: "call-1".to_owned(),
                        name: "echo".to_owned(),
                        input: json!({ "message": "hello" }),
                        operation_id: None,
                    }],
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
                    assert!(request.messages.iter().any(|message| {
                        message.role == ConversationRole::Assistant
                            && message.text() == "calling tool"
                            && message.tool_calls().iter().any(|call| call.id == "call-1")
                    }));
                    assert!(request.messages.iter().any(|message| {
                        message.role == ConversationRole::Tool
                            && tool_result_name(message) == Some("echo")
                            && message.text().contains("\"hello\"")
                    }));
                    Ok(ModelResponse {
                        attempts: Vec::new(),
                        provider: "mock".to_owned(),
                        model_id: "mock-model".to_owned(),
                        catalog_model_id: None,
                        thinking_level: None,
                        parts: response_parts("all done".to_owned(), Vec::new(), Vec::new()),
                        finish_reason: FinishReason::Stop,
                        structured_output: None,
                        usage: ModelUsage::default(),
                    })
                }
                .map(model_response_stream)
            }),
    )));
    let logger: Arc<dyn Logger> = Arc::new(Unimock::new((
        LoggerMock::log_model_call
            .next_call(matching!(_))
            .returns(Ok(())),
        LoggerMock::log_tool_activity
            .next_call(matching!(_))
            .answers(&|_, entry: &ToolActivityLogEntry| {
                assert_eq!(entry.tool_name, "echo");
                assert_eq!(entry.activity_verb.as_deref(), Some("Echoing"));
                assert_eq!(entry.phase, ToolActivityPhase::Started);
                Ok(())
            }),
        LoggerMock::log_tool_call.next_call(matching!(_)).answers(
            &|_, entry: &ToolCallLogEntry| {
                assert_eq!(entry.call.name, "echo");
                assert!(matches!(entry.result, ToolCallLogResult::Success { .. }));
                Ok(())
            },
        ),
        LoggerMock::log_tool_activity
            .next_call(matching!(_))
            .answers(&|_, entry: &ToolActivityLogEntry| {
                assert_eq!(entry.tool_name, "echo");
                assert_eq!(entry.activity_verb.as_deref(), Some("Echoing"));
                assert_eq!(entry.phase, ToolActivityPhase::Completed);
                Ok(())
            }),
        LoggerMock::log_model_call
            .next_call(matching!(_))
            .returns(Ok(())),
        LoggerMock::log_turn_outcome
            .next_call(matching!(_))
            .returns(Ok(())),
    )));
    let tool = TypedEchoTool::succeeding();
    let runtime =
        runtime_with_logger(model, logger, vec![tool.tool()]).expect("runtime should build");

    let mut turn = runtime.send(user_message("start"), Some(4));
    let first_step = turn.step().await.expect("first step should succeed");
    assert!(matches!(
        first_step,
        StepOutcome::Stepped {
            assistant_message,
            steps_taken: 1,
            ..
        } if assistant_message == "calling tool"
    ));
    assert_eq!(tool.parse_count(), 1);

    let outcome = turn.run().await.expect("run should complete");
    assert!(matches!(
        outcome,
        RunOutcome::Completed {
            assistant_message,
            steps_taken: 2,
        } if assistant_message == "all done"
    ));
    assert_eq!(turn.successful_tool_calls().len(), 1);
}
