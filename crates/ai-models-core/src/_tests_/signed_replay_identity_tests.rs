//! Signed provider replay state contributes to synthetic tool-call identity.

use ai_interface::{
    ConversationMessage, ConversationPart, ConversationRole, ModelRequest, ProviderId,
    ProviderState,
};
use serde_json::json;

use crate::synthetic_tool_call_scope;

#[test]
fn synthetic_scope_includes_signed_replay_content() {
    for (provider, mut context, pointer) in [
        (
            "anthropic",
            json!({ "blocks": [
                {"type": "thinking", "thinking": "", "signature": "first"}
            ]}),
            "/blocks/0/signature",
        ),
        (
            "google",
            json!({ "parts": [
                {"functionCall": {"name": "lookup", "args": {"id": 7}}, "thoughtSignature": "first"}
            ]}),
            "/parts/0/thoughtSignature",
        ),
    ] {
        let mut request = ModelRequest {
            system_prompt: String::new(),
            messages: vec![ConversationMessage::new(
                ConversationRole::Assistant,
                vec![state_part(provider, context.clone())],
            )],
            tools: Vec::new(),
            response_schema: None,
            controls: Default::default(),
        };
        let baseline = synthetic_tool_call_scope(&request);
        *context.pointer_mut(pointer).unwrap() = json!("second");
        request.messages[0].parts = vec![state_part(provider, context)];
        assert_ne!(synthetic_tool_call_scope(&request), baseline, "{pointer}");
    }
}

fn state_part(provider: &str, payload: serde_json::Value) -> ConversationPart {
    ConversationPart::reasoning("").with_provider_state(ProviderState {
        provider: ProviderId::from(provider.to_owned()),
        model: String::new(),
        version: 1,
        payload: payload.as_object().unwrap().clone(),
    })
}

#[test]
fn synthetic_scope_covers_state_owner_model_version_and_part_position() {
    let baseline_part = state_part("provider", json!({"signed": "opaque"}));
    let baseline = ModelRequest {
        system_prompt: String::new(),
        messages: vec![ConversationMessage::new(
            ConversationRole::Assistant,
            vec![baseline_part.clone(), ConversationPart::text("visible")],
        )],
        tools: Vec::new(),
        response_schema: None,
        controls: Default::default(),
    };
    let expected = synthetic_tool_call_scope(&baseline);
    for (field, value) in [
        ("provider", json!("other")),
        ("model", json!("different")),
        ("version", json!(2)),
    ] {
        let mut encoded = serde_json::to_value(&baseline_part).unwrap();
        encoded["provider_state"][field] = value;
        let mut request = baseline.clone();
        request.messages[0].parts[0] = serde_json::from_value(encoded).unwrap();
        assert_ne!(synthetic_tool_call_scope(&request), expected, "{field}");
    }
    let mut request = baseline.clone();
    request.messages[0].parts.reverse();
    assert_ne!(synthetic_tool_call_scope(&request), expected);
    assert_eq!(synthetic_tool_call_scope(&baseline), expected);
}
