//! MiniMax interleaved-thinking replay tests.

use ai_interface::{
    ConversationMessage, DeepSeekToolCallContext, MiniMaxReasoningDetail, Model, ModelRequest,
    OpenAiReasoningSummary, ProviderConversationItem, QwenToolCallContext,
};
use json_http::JsonHttpResponse;
use serde_json::json;

use super::{MiniMaxModel, support::recording_http_client};

#[tokio::test]
async fn preserves_and_replays_complete_reasoning_context_without_disclosure() {
    let (http_client, requests) =
        recording_http_client([reasoning_tool_response(), stopped_response()]);
    let model = MiniMaxModel::new(http_client, "MiniMax-M3", "minimax-key");
    let first_response = model
        .complete(&simple_request())
        .await
        .expect("reasoning tool response should parse");

    assert_eq!(first_response.assistant_message, "I will check.");
    assert!(
        !first_response
            .assistant_message
            .contains("private chain of thought")
    );
    assert_eq!(
        first_response.provider_context,
        vec![ProviderConversationItem::MiniMaxAssistant {
            reasoning_content: Some("private chain of thought".to_owned()),
            reasoning_details: vec![MiniMaxReasoningDetail {
                kind: Some("reasoning.text".to_owned()),
                id: Some("reasoning-1".to_owned()),
                format: Some("MiniMax-response-v1".to_owned()),
                index: Some(0),
                text: Some("private chain of thought".to_owned()),
            }],
        }]
    );

    let serialized = serde_json::to_value(&first_response.provider_context)
        .expect("provider context should serialize");
    let round_tripped =
        serde_json::from_value(serialized).expect("provider context should deserialize");
    let mut replay_context: Vec<ProviderConversationItem> = round_tripped;
    replay_context.push(ProviderConversationItem::OpenAiReasoning {
        id: "foreign".to_owned(),
        summary: vec![OpenAiReasoningSummary {
            kind: "summary_text".to_owned(),
            text: "foreign provider state".to_owned(),
        }],
        encrypted_content: None,
    });
    replay_context.push(ProviderConversationItem::DeepSeekAssistantMessage {
        content: "DeepSeek-owned content".to_owned(),
        reasoning_content: Some("DeepSeek-owned reasoning".to_owned()),
        tool_calls: vec![DeepSeekToolCallContext {
            id: "deepseek_call".to_owned(),
            name: "ignored".to_owned(),
            arguments: "{\"foreign\":true}".to_owned(),
        }],
    });
    replay_context.push(ProviderConversationItem::QwenAssistantMessage {
        content: Some("Qwen-owned content".to_owned()),
        reasoning_content: Some("Qwen-owned reasoning".to_owned()),
        tool_calls: vec![QwenToolCallContext {
            id: "qwen_call".to_owned(),
            name: "ignored".to_owned(),
            arguments: "{}".to_owned(),
        }],
    });
    model
        .complete(&ModelRequest {
            system_prompt: "system".to_owned(),
            messages: vec![
                ConversationMessage::assistant_with_provider_context(
                    first_response.assistant_message,
                    first_response.tool_calls,
                    replay_context,
                ),
                ConversationMessage::tool("tool result", "memory_read", "call_1"),
            ],
            tools: Vec::new(),
            response_schema: None,
        })
        .await
        .expect("continuation response should parse");

    let requests = requests
        .lock()
        .expect("requests lock should not be poisoned");
    let replay_body = requests[1]
        .body
        .as_ref()
        .and_then(|body| body.as_json())
        .expect("JSON body should be present");
    assert_eq!(
        replay_body["messages"][1],
        json!({
            "role": "assistant",
            "content": "I will check.",
            "reasoning_content": "private chain of thought",
            "reasoning_details": [{
                "type": "reasoning.text",
                "id": "reasoning-1",
                "format": "MiniMax-response-v1",
                "index": 0,
                "text": "private chain of thought"
            }],
            "tool_calls": [{
                "id": "call_1",
                "type": "function",
                "function": {
                    "name": "memory_read",
                    "arguments": "{\"path\":\"root\"}"
                }
            }]
        })
    );
    assert!(!replay_body.to_string().contains("foreign provider state"));
    assert!(!replay_body.to_string().contains("DeepSeek-owned"));
    assert!(!replay_body.to_string().contains("Qwen-owned"));
}

fn simple_request() -> ModelRequest {
    ModelRequest {
        system_prompt: "system".to_owned(),
        messages: vec![ConversationMessage::user("Check memory")],
        tools: Vec::new(),
        response_schema: None,
    }
}

fn reasoning_tool_response() -> JsonHttpResponse<serde_json::Value> {
    JsonHttpResponse {
        status: 200,
        body: json!({
            "choices": [{
                "finish_reason": "tool_calls",
                "message": {
                    "content": "I will check.",
                    "reasoning_content": "private chain of thought",
                    "reasoning_details": [{
                        "type": "reasoning.text",
                        "id": "reasoning-1",
                        "format": "MiniMax-response-v1",
                        "index": 0,
                        "text": "private chain of thought"
                    }],
                    "tool_calls": [{
                        "id": "call_1",
                        "type": "function",
                        "function": {
                            "name": "memory_read",
                            "arguments": "{\"path\":\"root\"}"
                        }
                    }]
                }
            }]
        }),
    }
}

fn stopped_response() -> JsonHttpResponse<serde_json::Value> {
    JsonHttpResponse {
        status: 200,
        body: json!({
            "choices": [{
                "finish_reason": "stop",
                "message": {"content": "Done"}
            }]
        }),
    }
}
