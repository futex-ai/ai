//! Provider replay parts survive caller-owned tool rounds.

use std::sync::Arc;

use ai_interface::{
    ConversationRole, Effort, FinishReason, Model, ModelMock, ModelResponse, ModelUsage, ToolCall,
};
use serde_json::json;
use unimock::{MockFn, Unimock, matching};

use crate::Agent;
use crate::tests::event_support::{collect_events, request};
use crate::tests::support::{
    TypedEchoTool, provider_reasoning, replay_parts, response_parts, runtime, stream_result,
    user_message,
};

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
    let mut conversation = vec![user_message("start")];
    collect_events(runtime.run(&mut conversation, request(Some(1))))
        .await
        .expect("turn should succeed");
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
    let mut conversation = vec![user_message("start")];
    collect_events(runtime.run(&mut conversation, request(Some(1))))
        .await
        .expect("turn should succeed");
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
