//! Signed provider replay state contributes to synthetic tool-call identity.

use ai_interface::{ConversationMessage, ModelRequest};
use serde_json::json;

use crate::synthetic_tool_call_scope;

#[test]
fn synthetic_scope_includes_signed_replay_content() {
    for (mut context, pointer) in [
        (
            json!({"type": "anthropic_assistant_message", "blocks": [
                {"type": "thinking", "thinking": "", "signature": "first"}
            ]}),
            "/blocks/0/signature",
        ),
        (
            json!({"type": "google_assistant_message", "parts": [
                {"functionCall": {"name": "lookup", "args": {"id": 7}}, "thoughtSignature": "first"}
            ]}),
            "/parts/0/thoughtSignature",
        ),
    ] {
        let mut request = ModelRequest {
            system_prompt: String::new(),
            messages: vec![ConversationMessage::assistant_with_provider_context(
                "",
                Vec::new(),
                vec![serde_json::from_value(context.clone()).unwrap()],
            )],
            tools: Vec::new(),
            response_schema: None,
            controls: Default::default(),
        };
        let baseline = synthetic_tool_call_scope(&request);
        *context.pointer_mut(pointer).unwrap() = json!("second");
        request.messages[0].provider_context = vec![serde_json::from_value(context).unwrap()];
        assert_ne!(synthetic_tool_call_scope(&request), baseline, "{pointer}");
    }
}
