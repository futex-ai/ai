//! Qwen request, multimodal, tool, replay, and structured-output tests.

use ai_interface::{
    ConversationMessage, ConversationPart, ConversationRole, Effort, ModelError, ModelRequest,
    StructuredOutputSchema, ToolCall, ToolDefinition,
};
use serde_json::{Value, json};

use crate::part_test_support::{assistant_with_items, message};

use super::catalog::{QWEN_3_7_FLASH_ID, QWEN_3_7_MAX_ID, QWEN_3_7_PLUS_ID};
use super::{request::build_request, test_support::simple_request};

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
    let body =
        request_json(QWEN_3_7_FLASH_ID, Effort::High, &request).expect("tool request should build");

    assert_eq!(body["tool_choice"], "auto");
    assert_eq!(body["parallel_tool_calls"], true);
    assert_eq!(body["tools"][0]["type"], "function");
    assert_eq!(body["tools"][0]["function"]["name"], "memory_read");
    assert_eq!(body["messages"][2]["content"], Value::Null);
    assert_eq!(body["messages"][2]["tool_calls"][0]["id"], "call_1");
    assert_eq!(body["messages"][3]["tool_call_id"], "call_1");
}

#[test]
fn serializes_empty_non_tool_assistant_content_as_a_string() {
    let mut request = simple_request();
    request
        .messages
        .push(ConversationMessage::assistant("", Vec::new()));

    let body = request_json(QWEN_3_7_PLUS_ID, Effort::High, &request)
        .expect("empty assistant history should build");

    assert_eq!(body["messages"][2]["content"], "");
    assert!(body["messages"][2].get("tool_calls").is_none());
}

#[test]
fn serializes_plus_and_flash_images_as_data_urls() {
    let request = request_with_user_parts();

    for model_id in [QWEN_3_7_PLUS_ID, QWEN_3_7_FLASH_ID] {
        let body =
            request_json(model_id, Effort::High, &request).expect("vision request should build");
        assert_eq!(body["messages"][1]["content"][0]["type"], "text");
        assert_eq!(
            body["messages"][1]["content"][1]["image_url"]["url"],
            "data:image/png;base64,abc123"
        );
    }
}

#[test]
fn rejects_max_images_and_non_user_content_parts() {
    let max_error = build_request(QWEN_3_7_MAX_ID, Effort::High, &request_with_user_parts())
        .expect_err("Max image input should fail");
    assert!(matches!(max_error, ModelError::Provider { .. }));

    let request = ModelRequest {
        system_prompt: "system".to_owned(),
        messages: vec![message(
            ConversationRole::Assistant,
            "fallback".to_owned(),
            vec![ConversationPart::image("image/png", "abc123")],
            None,
            None,
            Vec::new(),
        )],
        tools: Vec::new(),
        response_schema: None,
        controls: Default::default(),
    };
    let role_error = build_request(QWEN_3_7_PLUS_ID, Effort::High, &request)
        .expect_err("assistant content parts should fail");
    assert!(matches!(role_error, ModelError::Provider { .. }));
}

#[test]
fn replays_qwen_owned_raw_fields_and_ignores_foreign_context() {
    let assistant = assistant_with_items(
        "normalized",
        Vec::new(),
        vec![
            json!({"type": "deep_seek_assistant_message", "content": "foreign".to_owned(), "reasoning_content": Some("foreign reasoning".to_owned()), "tool_calls": vec![json!({"id": "foreign_call".to_owned(), "name": "ignored".to_owned(), "arguments": "{}".to_owned()})]}),
            json!({"type": "qwen_assistant_message", "content": serde_json::Value::Null, "reasoning_content": Some("private reasoning".to_owned()), "tool_calls": vec![json!({"id": "qwen_call".to_owned(), "name": "memory_read".to_owned(), "arguments": "{ \"path\": \"raw\" }".to_owned()})]}),
        ],
    );
    let request = ModelRequest {
        system_prompt: "system".to_owned(),
        messages: vec![assistant],
        tools: Vec::new(),
        response_schema: None,
        controls: Default::default(),
    };
    let body = request_json(QWEN_3_7_PLUS_ID, Effort::High, &request)
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
fn replays_null_non_tool_content_as_an_empty_string() {
    let assistant = assistant_with_items(
        "",
        Vec::new(),
        vec![
            json!({"type": "qwen_assistant_message", "content": serde_json::Value::Null, "reasoning_content": Some("private reasoning".to_owned()), "tool_calls": []}),
        ],
    );
    let request = ModelRequest {
        system_prompt: "system".to_owned(),
        messages: vec![assistant],
        tools: Vec::new(),
        response_schema: None,
        controls: Default::default(),
    };

    let body = request_json(QWEN_3_7_PLUS_ID, Effort::High, &request)
        .expect("non-tool replay request should build");
    let replay = &body["messages"][1];

    assert_eq!(replay["content"], "");
    assert_eq!(replay["reasoning_content"], "private reasoning");
    assert!(replay.get("tool_calls").is_none());
}

#[test]
fn structured_output_always_prompts_and_uses_native_json_only_when_supported() {
    let mut request = simple_request();
    request.response_schema = Some(status_schema());

    for (model_id, thinking_level, native) in [
        (QWEN_3_7_PLUS_ID, Effort::Disabled, true),
        (QWEN_3_7_FLASH_ID, Effort::Disabled, true),
        (QWEN_3_7_PLUS_ID, Effort::High, false),
        (QWEN_3_7_MAX_ID, Effort::Disabled, false),
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

#[test]
fn rejects_video_parts_with_typed_provider_error() {
    for tested_model in [QWEN_3_7_PLUS_ID, QWEN_3_7_FLASH_ID] {
        let request = ModelRequest {
            system_prompt: "system".to_owned(),
            messages: vec![ConversationMessage::user_with_parts(vec![
                ConversationPart::Video {
                    mime_type: "video/mp4".to_owned(),
                    data_base64: "dmlkZW8=".to_owned(),
                    provider_state: None,
                },
            ])],
            tools: Vec::new(),
            response_schema: None,
            controls: Default::default(),
        };

        let error = request_json(tested_model, Effort::Disabled, &request)
            .expect_err("video parts must be rejected");

        match error {
            ModelError::Provider {
                provider,
                model_id,
                message,
            } => {
                assert_eq!(provider, "qwen");
                assert_eq!(model_id, tested_model);
                assert_eq!(message, "Qwen accepts text and image content parts only");
            }
            other => panic!("unexpected error: {other:?}"),
        }
    }
}

fn request_with_user_parts() -> ModelRequest {
    ModelRequest {
        system_prompt: "system".to_owned(),
        messages: vec![ConversationMessage::user_with_parts(vec![
            ConversationPart::Text {
                text: "What is shown?".to_owned(),
                provider_state: None,
            },
            ConversationPart::Image {
                mime_type: "image/png".to_owned(),
                data_base64: "abc123".to_owned(),
                provider_state: None,
            },
        ])],
        tools: Vec::new(),
        response_schema: None,
        controls: Default::default(),
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
    thinking_level: Effort,
    request: &ModelRequest,
) -> Result<Value, ModelError> {
    let request = build_request(model_id, thinking_level, request)?;
    Ok(serde_json::to_value(request).expect("Qwen request should serialize"))
}
