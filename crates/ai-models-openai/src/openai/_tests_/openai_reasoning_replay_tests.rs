//! Tests for OpenAI Responses reasoning replay request mapping.

use ai_interface::{
    ConversationMessage, DeepSeekToolCallContext, KimiToolCallContext, Model, ModelRequest,
    OpenAiReasoningSummary, ProviderConversationItem, QwenToolCallContext, ToolCall,
};
use serde_json::json;

use super::OpenAiModel;
use crate::openai::stream_support::client_for_buffered_bodies;

#[tokio::test]
async fn replays_openai_reasoning_context_before_tool_outputs() {
    let (http_client, requests) = client_for_buffered_bodies(vec![openai_text_response("Done")]);
    let model = OpenAiModel::new(http_client, "gpt-5.5", "sk-openai");
    let assistant = ConversationMessage::assistant_with_provider_context(
        "",
        vec![ToolCall {
            id: "call_1".to_owned(),
            name: "memory_read".to_owned(),
            input: json!({"path": "root"}),
            operation_id: None,
        }],
        vec![
            ProviderConversationItem::DeepSeekAssistantMessage {
                content: "DeepSeek-owned content".to_owned(),
                reasoning_content: Some("DeepSeek-owned reasoning".to_owned()),
                tool_calls: vec![DeepSeekToolCallContext {
                    id: "deepseek_call".to_owned(),
                    name: "ignored".to_owned(),
                    arguments: "{\"foreign\":true}".to_owned(),
                }],
            },
            ProviderConversationItem::KimiAssistantMessage {
                content: None,
                reasoning_content: Some("Kimi-owned context".to_owned()),
                tool_calls: vec![KimiToolCallContext {
                    id: "kimi_call".to_owned(),
                    name: "ignored".to_owned(),
                    arguments: "{}".to_owned(),
                }],
            },
            ProviderConversationItem::QwenAssistantMessage {
                content: Some("Qwen-owned content".to_owned()),
                reasoning_content: Some("Qwen-owned reasoning".to_owned()),
                tool_calls: vec![QwenToolCallContext {
                    id: "qwen_call".to_owned(),
                    name: "ignored".to_owned(),
                    arguments: "{}".to_owned(),
                }],
            },
            ProviderConversationItem::OpenAiReasoning {
                id: "rs_1".to_owned(),
                summary: vec![OpenAiReasoningSummary {
                    kind: "summary_text".to_owned(),
                    text: "Need the memory tool.".to_owned(),
                }],
                encrypted_content: Some("encrypted-reasoning".to_owned()),
            },
            ProviderConversationItem::OpenAiFunctionCall {
                id: Some("fc_1".to_owned()),
                call_id: "call_1".to_owned(),
                name: "memory_read".to_owned(),
                arguments: "{\n  \"path\": \"root\"\n}".to_owned(),
            },
        ],
    );

    model
        .complete(&ModelRequest {
            system_prompt: "system".to_owned(),
            messages: vec![
                ConversationMessage::user("start"),
                assistant,
                ConversationMessage::tool("{\"ok\":true}", "memory_read", "call_1"),
            ],
            tools: Vec::new(),
            response_schema: None,
            controls: Default::default(),
        })
        .await
        .expect("OpenAI response should parse");

    let requests = requests
        .lock()
        .expect("requests lock should not be poisoned");
    let input = &requests[0].body.as_ref().expect("body present")["input"];
    assert_eq!(input[1]["type"], "reasoning");
    assert_eq!(input[1]["id"], "rs_1");
    assert_eq!(input[1]["summary"][0]["type"], "summary_text");
    assert_eq!(input[1]["encrypted_content"], "encrypted-reasoning");
    assert_eq!(input[2]["type"], "function_call");
    assert_eq!(input[2]["id"], "fc_1");
    assert_eq!(input[2]["call_id"], "call_1");
    assert_eq!(input[2]["arguments"], "{\n  \"path\": \"root\"\n}");
    assert_eq!(input[3]["type"], "function_call_output");
    assert_eq!(input[3]["call_id"], "call_1");
    assert_eq!(function_call_count(input), 1);
    assert!(!input.to_string().contains("DeepSeek-owned"));
    assert!(!input.to_string().contains("Qwen-owned"));
}

#[tokio::test]
async fn replays_assistant_message_phase_before_function_call_context() {
    let (http_client, requests) = client_for_buffered_bodies(vec![openai_text_response("Done")]);
    let model = OpenAiModel::new(http_client, "gpt-5.5", "sk-openai");
    let assistant = ConversationMessage::assistant_with_provider_context(
        "I'll inspect memory.",
        vec![ToolCall {
            id: "call_1".to_owned(),
            name: "memory_read".to_owned(),
            input: json!({"path": "root"}),
            operation_id: None,
        }],
        vec![
            ProviderConversationItem::OpenAiMessage {
                phase: Some("commentary".to_owned()),
            },
            ProviderConversationItem::OpenAiFunctionCall {
                id: Some("fc_1".to_owned()),
                call_id: "call_1".to_owned(),
                name: "memory_read".to_owned(),
                arguments: "{\"path\":\"root\"}".to_owned(),
            },
        ],
    );

    model
        .complete(&ModelRequest {
            system_prompt: "system".to_owned(),
            messages: vec![
                ConversationMessage::user("start"),
                assistant,
                ConversationMessage::tool("{\"ok\":true}", "memory_read", "call_1"),
            ],
            tools: Vec::new(),
            response_schema: None,
            controls: Default::default(),
        })
        .await
        .expect("OpenAI response should parse");

    let requests = requests
        .lock()
        .expect("requests lock should not be poisoned");
    let input = &requests[0].body.as_ref().expect("body present")["input"];
    assert_eq!(input[1]["role"], "assistant");
    assert_eq!(input[1]["phase"], "commentary");
    assert_eq!(input[1]["content"], "I'll inspect memory.");
    assert_eq!(input[2]["type"], "function_call");
    assert_eq!(input[2]["id"], "fc_1");
    assert_eq!(input[3]["type"], "function_call_output");
    assert_eq!(function_call_count(input), 1);
}

#[tokio::test]
async fn replays_phase_less_assistant_message_before_function_call_context() {
    let (http_client, requests) = client_for_buffered_bodies(vec![openai_text_response("Done")]);
    let model = OpenAiModel::new(http_client, "gpt-5.5", "sk-openai");
    let assistant = ConversationMessage::assistant_with_provider_context(
        "I'll inspect memory.",
        vec![ToolCall {
            id: "call_1".to_owned(),
            name: "memory_read".to_owned(),
            input: json!({"path": "root"}),
            operation_id: None,
        }],
        vec![
            ProviderConversationItem::OpenAiMessage { phase: None },
            ProviderConversationItem::OpenAiFunctionCall {
                id: Some("fc_1".to_owned()),
                call_id: "call_1".to_owned(),
                name: "memory_read".to_owned(),
                arguments: "{\"path\":\"root\"}".to_owned(),
            },
        ],
    );

    model
        .complete(&ModelRequest {
            system_prompt: "system".to_owned(),
            messages: vec![
                ConversationMessage::user("start"),
                assistant,
                ConversationMessage::tool("{\"ok\":true}", "memory_read", "call_1"),
            ],
            tools: Vec::new(),
            response_schema: None,
            controls: Default::default(),
        })
        .await
        .expect("OpenAI response should parse");

    let requests = requests
        .lock()
        .expect("requests lock should not be poisoned");
    let input = &requests[0].body.as_ref().expect("body present")["input"];
    let assistant_message = input[1]
        .as_object()
        .expect("assistant message should serialize as an object");
    assert_eq!(input[1]["role"], "assistant");
    assert_eq!(input[1]["content"], "I'll inspect memory.");
    assert!(!assistant_message.contains_key("phase"));
    assert_eq!(input[2]["type"], "function_call");
    assert_eq!(input[2]["id"], "fc_1");
    assert_eq!(input[3]["type"], "function_call_output");
    assert_eq!(function_call_count(input), 1);
}

fn function_call_count(input: &serde_json::Value) -> usize {
    input
        .as_array()
        .expect("input array")
        .iter()
        .filter(|item| item["type"] == "function_call")
        .count()
}

fn openai_text_response(text: &str) -> serde_json::Value {
    json!({
        "status": "completed",
        "output": [{
            "type": "message",
            "role": "assistant",
            "content": [{ "type": "output_text", "text": text }]
        }],
        "usage": {
            "input_tokens": 120,
            "output_tokens": 32,
            "total_tokens": 152
        }
    })
}
