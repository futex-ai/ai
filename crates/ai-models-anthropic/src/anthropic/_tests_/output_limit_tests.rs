//! Catalog-aware output defaults and ceilings on final transport requests.

use ai_interface::{ConversationMessage, Effort, Model, ModelRequest};
use serde_json::json;

use crate::{AnthropicModel, CLAUDE_HAIKU_4_5, CLAUDE_OPUS_4_7, CLAUDE_OPUS_5, CLAUDE_SONNET_5};

use super::stream_support::{client_for_buffered_bodies, custom_spec};

#[tokio::test]
async fn output_limits_use_catalog_defaults_ceilings_and_unknown_fallback() {
    for (spec, requested, expected) in [
        (CLAUDE_OPUS_5, None, 128_000),
        (CLAUDE_SONNET_5, Some(9_000), 9_000),
        (CLAUDE_OPUS_4_7, Some(200_000), 128_000),
        (CLAUDE_HAIKU_4_5, None, 64_000),
        (CLAUDE_HAIKU_4_5, Some(100_000), 64_000),
        (custom_spec("custom-provider-id"), None, 64_000),
        (custom_spec("custom-provider-id"), Some(100_000), 64_000),
        (custom_spec("custom-provider-id"), Some(8_000), 8_000),
    ] {
        let text_id = spec.text_id();
        let (client, requests) = client_for_buffered_bodies(vec![json!({
            "content": [{"type": "text", "text": "Done"}], "stop_reason": "end_turn"
        })]);
        let model = AnthropicModel::new(client, spec, "key");
        let mut request = ModelRequest {
            system_prompt: String::new(),
            messages: vec![ConversationMessage::user("hello")],
            tools: Vec::new(),
            response_schema: None,
            controls: Default::default(),
        };
        request.controls.generation.effort = Some(Effort::Max);
        request.controls.generation.max_output_tokens = requested;
        model.complete(&request).await.unwrap();
        let requests = requests.lock().unwrap();
        assert_eq!(
            serde_json::from_slice::<serde_json::Value>(
                requests[0]
                    .body
                    .as_ref()
                    .unwrap()
                    .as_json_bytes()
                    .expect("JSON request body")
            )
            .expect("valid JSON request body")["max_tokens"],
            expected,
            "{text_id}, requested {requested:?}"
        );
    }
}
