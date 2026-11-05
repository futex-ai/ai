use ai_interface::{
    ConversationMessage, ConversationPart, ConversationRole, ModelRequest, ProviderId,
    ProviderState,
};

use crate::{synthetic_tool_call_id, synthetic_tool_call_scope};

#[test]
fn portable_message_keeps_its_version_one_synthetic_scope() {
    assert_eq!(
        synthetic_tool_call_scope(&request_with_message("hello")),
        "2e68385026f1eafd"
    );
}

#[test]
fn synthetic_scope_changes_with_request_history() {
    let first = synthetic_tool_call_scope(&request_with_message("first request"));
    let second = synthetic_tool_call_scope(&request_with_message("second request"));

    assert_ne!(first, second);
}

#[test]
fn synthetic_tool_call_id_changes_with_arguments() {
    let scope = synthetic_tool_call_scope(&request_with_message("same request"));
    let first = synthetic_tool_call_id(
        "provider_tool_call:",
        &scope,
        0,
        "memory_read",
        "{\"path\":\"one\"}",
    );
    let second = synthetic_tool_call_id(
        "provider_tool_call:",
        &scope,
        0,
        "memory_read",
        "{\"path\":\"two\"}",
    );

    assert_ne!(first, second);
}

#[test]
fn synthetic_scope_distinguishes_video_parts_from_image_parts() {
    let image_scope = synthetic_tool_call_scope(&request_with_part(ConversationPart::Image {
        mime_type: "video/mp4".to_owned(),
        data_base64: "c2FtZQ==".to_owned(),
        provider_state: None,
    }));
    let video_scope = synthetic_tool_call_scope(&request_with_part(ConversationPart::Video {
        mime_type: "video/mp4".to_owned(),
        data_base64: "c2FtZQ==".to_owned(),
        provider_state: None,
    }));

    assert_ne!(image_scope, video_scope);
}

#[test]
fn synthetic_scope_changes_with_video_part_fields() {
    let baseline = synthetic_tool_call_scope(&request_with_part(ConversationPart::Video {
        mime_type: "video/mp4".to_owned(),
        data_base64: "b25l".to_owned(),
        provider_state: None,
    }));
    let variants = [
        ConversationPart::Video {
            mime_type: "video/webm".to_owned(),
            data_base64: "b25l".to_owned(),
            provider_state: None,
        },
        ConversationPart::Video {
            mime_type: "video/mp4".to_owned(),
            data_base64: "dHdv".to_owned(),
            provider_state: None,
        },
    ];

    for variant in variants {
        assert_ne!(
            synthetic_tool_call_scope(&request_with_part(variant)),
            baseline
        );
    }
}

#[test]
fn synthetic_scope_hashes_every_kimi_replay_field() {
    let baseline = kimi_request(
        Some("visible"),
        Some("reasoning"),
        "call_1",
        "memory_read",
        "{\"path\":\"one\"}",
    );
    let baseline_scope = synthetic_tool_call_scope(&baseline);
    let variants = [
        kimi_request(
            Some("changed"),
            Some("reasoning"),
            "call_1",
            "memory_read",
            "{\"path\":\"one\"}",
        ),
        kimi_request(
            Some("visible"),
            Some("changed"),
            "call_1",
            "memory_read",
            "{\"path\":\"one\"}",
        ),
        kimi_request(
            Some("visible"),
            Some("reasoning"),
            "call_2",
            "memory_read",
            "{\"path\":\"one\"}",
        ),
        kimi_request(
            Some("visible"),
            Some("reasoning"),
            "call_1",
            "memory_write",
            "{\"path\":\"one\"}",
        ),
        kimi_request(
            Some("visible"),
            Some("reasoning"),
            "call_1",
            "memory_read",
            "{\"path\":\"two\"}",
        ),
    ];

    for variant in variants {
        assert_ne!(synthetic_tool_call_scope(&variant), baseline_scope);
    }
}

#[test]
fn synthetic_scope_hashes_every_deepseek_replay_field() {
    let baseline = deepseek_request(
        "visible",
        Some("reasoning"),
        "call_1",
        "memory_read",
        "{\"path\":\"one\"}",
    );
    let baseline_scope = synthetic_tool_call_scope(&baseline);
    let variants = [
        deepseek_request(
            "changed",
            Some("reasoning"),
            "call_1",
            "memory_read",
            "{\"path\":\"one\"}",
        ),
        deepseek_request(
            "visible",
            Some("changed"),
            "call_1",
            "memory_read",
            "{\"path\":\"one\"}",
        ),
        deepseek_request(
            "visible",
            Some("reasoning"),
            "call_2",
            "memory_read",
            "{\"path\":\"one\"}",
        ),
        deepseek_request(
            "visible",
            Some("reasoning"),
            "call_1",
            "memory_write",
            "{\"path\":\"one\"}",
        ),
        deepseek_request(
            "visible",
            Some("reasoning"),
            "call_1",
            "memory_read",
            "{\"path\":\"two\"}",
        ),
    ];

    for variant in variants {
        assert_ne!(synthetic_tool_call_scope(&variant), baseline_scope);
    }
}

fn request_with_message(content: &str) -> ModelRequest {
    ModelRequest {
        system_prompt: "system".to_owned(),
        messages: vec![ConversationMessage::user(content)],
        tools: Vec::new(),
        response_schema: None,
        controls: Default::default(),
    }
}

fn request_with_part(part: ConversationPart) -> ModelRequest {
    ModelRequest {
        system_prompt: "system".to_owned(),
        messages: vec![ConversationMessage::user_with_parts(vec![part])],
        tools: Vec::new(),
        response_schema: None,
        controls: Default::default(),
    }
}

fn kimi_request(
    content: Option<&str>,
    reasoning_content: Option<&str>,
    id: &str,
    name: &str,
    arguments: &str,
) -> ModelRequest {
    request_with_state(
        "kimi",
        serde_json::json!({
            "content": content, "reasoning_content": reasoning_content,
            "tool_calls": [{"id": id, "name": name, "arguments": arguments}],
        }),
    )
}

fn deepseek_request(
    content: &str,
    reasoning_content: Option<&str>,
    id: &str,
    name: &str,
    arguments: &str,
) -> ModelRequest {
    request_with_state(
        "deepseek",
        serde_json::json!({
            "content": content, "reasoning_content": reasoning_content,
            "tool_calls": [{"id": id, "name": name, "arguments": arguments}],
        }),
    )
}

fn request_with_state(provider: &str, payload: serde_json::Value) -> ModelRequest {
    ModelRequest {
        system_prompt: "system".to_owned(),
        messages: vec![ConversationMessage::new(
            ConversationRole::Assistant,
            vec![
                ConversationPart::reasoning("").with_provider_state(ProviderState {
                    provider: ProviderId::from(provider.to_owned()),
                    model: String::new(),
                    version: 1,
                    payload: payload.as_object().unwrap().clone(),
                }),
            ],
        )],
        tools: Vec::new(),
        response_schema: None,
        controls: Default::default(),
    }
}
