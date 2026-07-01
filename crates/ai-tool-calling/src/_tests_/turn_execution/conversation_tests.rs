use std::sync::Arc;

use ai_interface::{
    ConversationRole, FinishReason, Logger, LoggerMock, Model, ModelMock, ModelRequest,
    ModelResponse, ModelUsage, OpenAiReasoningSummary, ProviderConversationItem,
    ToolActivityLogEntry, ToolActivityPhase, ToolCall, ToolCallLogEntry, ToolCallLogResult,
};
use serde_json::json;
use unimock::{MockFn, Unimock, matching};

use crate::{RunOutcome, StepOutcome, Turn};

use super::super::support::{TypedEchoTool, runtime, runtime_with_logger, user_message};

#[tokio::test]
async fn retained_conversation_is_used_across_future_sends() {
    let model: Arc<dyn Model> = Arc::new(Unimock::new((
        ModelMock::complete
            .next_call(matching!(_))
            .returns(Ok(ModelResponse {
                provider: "mock".to_owned(),
                model_id: "mock-model".to_owned(),
                catalog_model_id: None,
                thinking_level: None,
                assistant_message: "first response".to_owned(),
                tool_calls: Vec::new(),
                finish_reason: FinishReason::Stop,
                structured_output: None,
                provider_context: Vec::new(),
                usage: ModelUsage::default(),
            })),
        ModelMock::complete
            .next_call(matching!(_))
            .answers(&|_, request: &ModelRequest| {
                assert!(request.messages.iter().any(|message| {
                    message.role == ConversationRole::Assistant
                        && message.content == "first response"
                }));
                Ok(ModelResponse {
                    provider: "mock".to_owned(),
                    model_id: "mock-model".to_owned(),
                    catalog_model_id: None,
                    thinking_level: None,
                    assistant_message: "second response".to_owned(),
                    tool_calls: Vec::new(),
                    finish_reason: FinishReason::Stop,
                    structured_output: None,
                    provider_context: Vec::new(),
                    usage: ModelUsage::default(),
                })
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
    let provider_context = vec![ProviderConversationItem::OpenAiReasoning {
        id: "rs_1".to_owned(),
        summary: vec![OpenAiReasoningSummary {
            kind: "summary_text".to_owned(),
            text: "Need the tool.".to_owned(),
        }],
        encrypted_content: Some("encrypted-reasoning".to_owned()),
    }];
    let model: Arc<dyn Model> = Arc::new(Unimock::new(
        ModelMock::complete
            .next_call(matching!(_))
            .returns(Ok(ModelResponse {
                provider: "openai".to_owned(),
                model_id: "gpt-5.5".to_owned(),
                catalog_model_id: None,
                thinking_level: Some("extra_high".to_owned()),
                assistant_message: "calling tool".to_owned(),
                tool_calls: vec![ToolCall {
                    id: "call-1".to_owned(),
                    name: "echo".to_owned(),
                    input: json!({ "message": "hello" }),
                    operation_id: None,
                }],
                finish_reason: FinishReason::ToolCalls,
                structured_output: None,
                provider_context: provider_context.clone(),
                usage: ModelUsage::default(),
            })),
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
    assert_eq!(assistant.provider_context, provider_context);
}

#[tokio::test]
async fn step_and_run_handle_tool_rounds_and_logger_callbacks() {
    let model: Arc<dyn Model> = Arc::new(Unimock::new((
        ModelMock::complete
            .next_call(matching!(_))
            .returns(Ok(ModelResponse {
                provider: "mock".to_owned(),
                model_id: "mock-model".to_owned(),
                catalog_model_id: None,
                thinking_level: None,
                assistant_message: "calling tool".to_owned(),
                tool_calls: vec![ToolCall {
                    id: "call-1".to_owned(),
                    name: "echo".to_owned(),
                    input: json!({ "message": "hello" }),
                    operation_id: None,
                }],
                finish_reason: FinishReason::ToolCalls,
                structured_output: None,
                provider_context: Vec::new(),
                usage: ModelUsage::default(),
            })),
        ModelMock::complete
            .next_call(matching!(_))
            .answers(&|_, request: &ModelRequest| {
                assert!(request.messages.iter().any(|message| {
                    message.role == ConversationRole::Assistant
                        && message.content == "calling tool"
                        && message.tool_calls.iter().any(|call| call.id == "call-1")
                }));
                assert!(request.messages.iter().any(|message| {
                    message.role == ConversationRole::Tool
                        && message.name.as_deref() == Some("echo")
                        && message.content.contains("\"hello\"")
                }));
                Ok(ModelResponse {
                    provider: "mock".to_owned(),
                    model_id: "mock-model".to_owned(),
                    catalog_model_id: None,
                    thinking_level: None,
                    assistant_message: "all done".to_owned(),
                    tool_calls: Vec::new(),
                    finish_reason: FinishReason::Stop,
                    structured_output: None,
                    provider_context: Vec::new(),
                    usage: ModelUsage::default(),
                })
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
