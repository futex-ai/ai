//! Included converters restore native part order and preserve absent provider state.

use ai_interface::ConversationPart;
use serde_json::json;

use crate::Registry;

use super::support::unused_http;

#[test]
fn converter_composes_included_providers_and_keeps_absent_state() {
    let converter = Registry::new(unused_http()).converter();
    for (provider, enabled, items) in [
        (
            "anthropic",
            cfg!(feature = "anthropic"),
            json!([{
                "type": "anthropic_assistant_message", "blocks": [
                    {"type": "text", "text": "answer"},
                    {"type": "thinking", "thinking": "reason", "signature": "sig"}
                ]
            }]),
        ),
        (
            "google",
            cfg!(feature = "google"),
            json!([{
                "type": "google_assistant_message", "parts": [
                    {"text": "answer", "thoughtSignature": "sig"}, {"text": "reason", "thought": true}
                ]
            }]),
        ),
        (
            "openai",
            cfg!(feature = "openai"),
            json!([
                {"type": "openai_message", "item": {"id": "msg", "type": "message"}},
                {"type": "openai_reasoning", "item": {"id": "rs", "type": "reasoning"}}
            ]),
        ),
    ] {
        let message = converter
            .read(json!({
                "role": "assistant", "content": "answer", "provider_context": items
            }))
            .unwrap();
        assert_eq!(message.parts.len(), 2, "{provider}");
        assert_eq!(
            matches!(message.parts[0], ConversationPart::Text { .. }),
            enabled,
            "{provider}"
        );
        let state = message.parts[0].provider_state().unwrap();
        assert_eq!(state.provider.as_str(), provider);
        let stored = serde_json::to_value(&message).unwrap();
        assert_eq!(converter.read(stored).unwrap(), message);
    }

    let message = converter
        .read(json!({
            "role": "assistant", "content": "answer", "provider_context": [{
                "type": "future_assistant_message", "token": "opaque", "nested": {"key": 5}
            }]
        }))
        .unwrap();
    assert_eq!(
        serde_json::to_value(&message.parts[0]).unwrap(),
        json!({
            "type": "reasoning", "text": "", "provider_state": {
                "provider": "future", "model": "", "version": 1,
                "payload": {"token": "opaque", "nested": {"key": 5}}
            }
        })
    );
}
