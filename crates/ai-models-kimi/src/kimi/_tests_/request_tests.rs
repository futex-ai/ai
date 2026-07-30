//! Kimi one-turn request mapping tests.

use ai_interface::{
    ConversationContentPart, ConversationMessage, ConversationRole, DeepSeekToolCallContext,
    ModelRequest, OpenAiReasoningSummary, ProviderConversationItem, ToolCall,
};
use serde_json::{Value, json};

use crate::KIMI_K3;

use super::{client::KimiReasoningEffort, request::build_request, test_support::simple_request};

#[test]
fn serializes_leading_system_and_every_shared_role() {
    let request = ModelRequest {
        system_prompt: "Be concise.".to_owned(),
        messages: vec![
            named_message(ConversationRole::User, "hello", "caller"),
            named_message(ConversationRole::Assistant, "checking", "agent"),
            ConversationMessage::tool("{\"ok\":true}", "memory_read", "call_1"),
        ],
        tools: Vec::new(),
        response_schema: None,
    };
    let body = request_json(&request, KimiReasoningEffort::Max);
    let messages = body["messages"].as_array().expect("messages array");

    assert_eq!(
        messages[0],
        json!({"role": "system", "content": "Be concise."})
    );
    assert_eq!(messages[1]["role"], "user");
    assert_eq!(messages[1]["content"], "hello");
    assert_eq!(messages[1]["name"], "caller");
    assert_eq!(messages[2]["role"], "assistant");
    assert_eq!(messages[2]["content"], "checking");
    assert_eq!(messages[2]["name"], "agent");
    assert_eq!(messages[3]["role"], "tool");
    assert_eq!(messages[3]["content"], "{\"ok\":true}");
    assert_eq!(messages[3]["tool_call_id"], "call_1");
    assert!(messages[3].get("name").is_none());
}

#[test]
fn serializes_empty_user_and_tool_content_as_strings() {
    let request = ModelRequest {
        system_prompt: "system".to_owned(),
        messages: vec![
            ConversationMessage::user(""),
            ConversationMessage::tool("", "memory_read", "call_1"),
        ],
        tools: Vec::new(),
        response_schema: None,
    };
    let body = request_json(&request, KimiReasoningEffort::Max);
    let messages = body["messages"].as_array().expect("messages array");

    assert_eq!(messages[1]["role"], "user");
    assert_eq!(messages[1]["content"], "");
    assert_eq!(messages[2]["role"], "tool");
    assert_eq!(messages[2]["content"], "");
    assert_eq!(messages[2]["tool_call_id"], "call_1");
}

#[test]
fn serializes_text_and_image_content_parts_as_data_urls() {
    let request = ModelRequest {
        system_prompt: "system".to_owned(),
        messages: vec![ConversationMessage::user_with_parts(
            "fallback",
            vec![
                ConversationContentPart::Text {
                    text: "What is shown?".to_owned(),
                },
                ConversationContentPart::Image {
                    mime_type: "image/png".to_owned(),
                    data_base64: "abc123".to_owned(),
                },
            ],
        )],
        tools: Vec::new(),
        response_schema: None,
    };
    let body = request_json(&request, KimiReasoningEffort::Max);

    assert_eq!(body["messages"][1]["content"][0]["type"], "text");
    assert_eq!(body["messages"][1]["content"][0]["text"], "What is shown?");
    assert_eq!(body["messages"][1]["content"][1]["type"], "image_url");
    assert_eq!(
        body["messages"][1]["content"][1]["image_url"]["url"],
        "data:image/png;base64,abc123"
    );
}

#[test]
fn ignores_foreign_provider_context_and_uses_normalized_assistant_fields() {
    let assistant = ConversationMessage::assistant_with_provider_context(
        "normalized",
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
            ProviderConversationItem::OpenAiMessage {
                phase: Some("commentary".to_owned()),
            },
            ProviderConversationItem::OpenAiReasoning {
                id: "reasoning_1".to_owned(),
                summary: vec![OpenAiReasoningSummary {
                    kind: "summary_text".to_owned(),
                    text: "private".to_owned(),
                }],
                encrypted_content: None,
            },
            ProviderConversationItem::XaiLegacyFunctionCall {
                tool_call_id: "xai_call".to_owned(),
                name: "ignored".to_owned(),
                arguments: "{\"foreign\":true}".to_owned(),
            },
        ],
    );
    let request = ModelRequest {
        system_prompt: "system".to_owned(),
        messages: vec![assistant],
        tools: Vec::new(),
        response_schema: None,
    };
    let body = request_json(&request, KimiReasoningEffort::Max);
    let assistant = &body["messages"][1];

    assert_eq!(assistant["content"], "normalized");
    assert_eq!(assistant["tool_calls"][0]["id"], "call_1");
    assert_eq!(
        assistant["tool_calls"][0]["function"]["arguments"],
        "{\"path\":\"root\"}"
    );
    assert!(assistant.get("reasoning_content").is_none());
    assert!(!body.to_string().contains("DeepSeek-owned"));
}

#[test]
fn maps_exact_reasoning_effort_and_omits_unsupported_fields() {
    let request = simple_request();
    for (effort, expected) in [
        (KimiReasoningEffort::Low, "low"),
        (KimiReasoningEffort::High, "high"),
        (KimiReasoningEffort::Max, "max"),
    ] {
        let body = request_json(&request, effort);
        let object = body.as_object().expect("request object");

        assert_eq!(body["reasoning_effort"], expected);
        for omitted in [
            "temperature",
            "top_p",
            "n",
            "presence_penalty",
            "frequency_penalty",
            "thinking",
            "stream",
            "partial",
            "file",
            "video",
            "prompt_cache_key",
            "safety_identifier",
        ] {
            assert!(!object.contains_key(omitted), "unexpected `{omitted}`");
        }
    }
}

fn named_message(role: ConversationRole, content: &str, name: &str) -> ConversationMessage {
    ConversationMessage {
        role,
        content: content.to_owned(),
        content_parts: Vec::new(),
        name: Some(name.to_owned()),
        tool_call_id: None,
        tool_calls: Vec::new(),
        provider_context: Vec::new(),
    }
}

fn request_json(request: &ModelRequest, effort: KimiReasoningEffort) -> Value {
    serde_json::to_value(build_request(KIMI_K3, effort, request))
        .expect("Kimi request should serialize")
}
