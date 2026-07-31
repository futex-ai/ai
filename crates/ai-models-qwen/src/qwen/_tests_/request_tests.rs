//! Qwen request, multimodal, tool, replay, and structured-output tests.

use ai_interface::{
    ConversationContentPart, ConversationMessage, ConversationRole, DeepSeekToolCallContext,
    ModelError, ModelRequest, ProviderConversationItem, QwenToolCallContext,
    StructuredOutputSchema, ToolCall, ToolDefinition,
};
use ai_models_core::ThinkingLevel;
use serde_json::{Value, json};

use crate::{QWEN_3_7_FLASH, QWEN_3_7_MAX, QWEN_3_7_PLUS};

use super::{request::build_request, test_support::simple_request};

#[test]
fn serializes_system_roles_and_exact_thinking_controls() {
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

    for (thinking_level, enabled) in [
        (ThinkingLevel::High, true),
        (ThinkingLevel::Disabled, false),
    ] {
        let body = request_json(QWEN_3_7_PLUS, thinking_level, &request)
            .expect("supported request should build");
        let messages = body["messages"].as_array().expect("messages array");

        assert_eq!(
            messages[0],
            json!({"role": "system", "content": "Be concise."})
        );
        assert_eq!(messages[1], json!({"role": "user", "content": "hello"}));
        assert_eq!(
            messages[2],
            json!({"role": "assistant", "content": "checking"})
        );
        assert_eq!(messages[3]["role"], "tool");
        assert_eq!(messages[3]["content"], "{\"ok\":true}");
        assert_eq!(messages[3]["tool_call_id"], "call_1");
        assert_eq!(body["stream"], false);
        assert_eq!(body["enable_thinking"], enabled);
        assert_eq!(body["preserve_thinking"], enabled);
    }
}

#[test]
fn serializes_tools_parallel_calls_and_matching_results() {
    let mut request = simple_request();
    request.tools = vec![ToolDefinition {
        name: "memory_read".to_owned(),
        description: "Read retained memory".to_owned(),
        input_schema: json!({
            "type": "object",
            "properties": {"path": {"type": "string"}},
            "required": ["path"]
        }),
        activity_verb: Some("Remembering".to_owned()),
    }];
    request.messages.push(ConversationMessage::assistant(
        "",
        vec![ToolCall {
            id: "call_1".to_owned(),
            name: "memory_read".to_owned(),
            input: json!({"path": "one"}),
            operation_id: None,
        }],
    ));
    request
        .messages
        .push(ConversationMessage::tool("result", "memory_read", "call_1"));
    let body = request_json(QWEN_3_7_FLASH, ThinkingLevel::High, &request)
        .expect("tool request should build");

    assert_eq!(body["tool_choice"], "auto");
    assert_eq!(body["parallel_tool_calls"], true);
    assert_eq!(body["tools"][0]["type"], "function");
    assert_eq!(body["tools"][0]["function"]["name"], "memory_read");
    assert_eq!(body["messages"][2]["content"], Value::Null);
    assert_eq!(body["messages"][2]["tool_calls"][0]["id"], "call_1");
    assert_eq!(body["messages"][3]["tool_call_id"], "call_1");
}

#[test]
fn serializes_plus_and_flash_images_as_data_urls() {
    let request = request_with_user_parts();

    for model_id in [QWEN_3_7_PLUS, QWEN_3_7_FLASH] {
        let body = request_json(model_id, ThinkingLevel::High, &request)
            .expect("vision request should build");
        assert_eq!(body["messages"][1]["content"][0]["type"], "text");
        assert_eq!(
            body["messages"][1]["content"][1]["image_url"]["url"],
            "data:image/png;base64,abc123"
        );
    }
}

#[test]
fn rejects_max_images_and_non_user_content_parts() {
    let max_error = build_request(
        QWEN_3_7_MAX,
        ThinkingLevel::High,
        &request_with_user_parts(),
    )
    .expect_err("Max image input should fail");
    assert!(matches!(max_error, ModelError::Provider { .. }));

    let request = ModelRequest {
        system_prompt: "system".to_owned(),
        messages: vec![ConversationMessage {
            role: ConversationRole::Assistant,
            content: "fallback".to_owned(),
            content_parts: vec![ConversationContentPart::Text {
                text: "typed".to_owned(),
            }],
            name: None,
            tool_call_id: None,
            tool_calls: Vec::new(),
            provider_context: Vec::new(),
        }],
        tools: Vec::new(),
        response_schema: None,
    };
    let role_error = build_request(QWEN_3_7_PLUS, ThinkingLevel::High, &request)
        .expect_err("assistant content parts should fail");
    assert!(matches!(role_error, ModelError::Provider { .. }));
}

#[test]
fn replays_qwen_owned_raw_fields_and_ignores_foreign_context() {
    let assistant = ConversationMessage::assistant_with_provider_context(
        "normalized",
        Vec::new(),
        vec![
            ProviderConversationItem::DeepSeekAssistantMessage {
                content: "foreign".to_owned(),
                reasoning_content: Some("foreign reasoning".to_owned()),
                tool_calls: vec![DeepSeekToolCallContext {
                    id: "foreign_call".to_owned(),
                    name: "ignored".to_owned(),
                    arguments: "{}".to_owned(),
                }],
            },
            ProviderConversationItem::QwenAssistantMessage {
                content: None,
                reasoning_content: Some("private reasoning".to_owned()),
                tool_calls: vec![QwenToolCallContext {
                    id: "qwen_call".to_owned(),
                    name: "memory_read".to_owned(),
                    arguments: "{ \"path\": \"raw\" }".to_owned(),
                }],
            },
        ],
    );
    let request = ModelRequest {
        system_prompt: "system".to_owned(),
        messages: vec![assistant],
        tools: Vec::new(),
        response_schema: None,
    };
    let body = request_json(QWEN_3_7_PLUS, ThinkingLevel::High, &request)
        .expect("replay request should build");
    let replay = &body["messages"][1];

    assert_eq!(replay["content"], Value::Null);
    assert_eq!(replay["reasoning_content"], "private reasoning");
    assert_eq!(replay["tool_calls"][0]["id"], "qwen_call");
    assert_eq!(
        replay["tool_calls"][0]["function"]["arguments"],
        "{ \"path\": \"raw\" }"
    );
    assert!(!body.to_string().contains("foreign"));
}

#[test]
fn structured_output_always_prompts_and_uses_native_json_only_when_supported() {
    let mut request = simple_request();
    request.response_schema = Some(status_schema());

    for (model_id, thinking_level, native) in [
        (QWEN_3_7_PLUS, ThinkingLevel::Disabled, true),
        (QWEN_3_7_FLASH, ThinkingLevel::Disabled, true),
        (QWEN_3_7_PLUS, ThinkingLevel::High, false),
        (QWEN_3_7_MAX, ThinkingLevel::Disabled, false),
    ] {
        let body = request_json(model_id, thinking_level, &request)
            .expect("structured request should build");
        let prompt = body["messages"][0]["content"]
            .as_str()
            .expect("system prompt text");

        assert!(prompt.contains("raw JSON"));
        assert!(prompt.contains("JSON Schema"));
        assert!(prompt.contains("status"));
        assert_eq!(
            body.get("response_format"),
            native.then_some(&json!({"type": "json_object"}))
        );
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

fn request_with_user_parts() -> ModelRequest {
    ModelRequest {
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
    }
}

fn status_schema() -> StructuredOutputSchema {
    StructuredOutputSchema {
        name: "status".to_owned(),
        schema: json!({"type": "object", "required": ["done"]}),
    }
}

fn request_json(
    model_id: &str,
    thinking_level: ThinkingLevel,
    request: &ModelRequest,
) -> Result<Value, ModelError> {
    let request = build_request(model_id, thinking_level, request)?;
    Ok(serde_json::to_value(request).expect("Qwen request should serialize"))
}
