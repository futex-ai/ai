//! Catalog-aware output defaults and ceilings on final transport requests.

use std::sync::Arc;

use ai_interface::{ConversationMessage, Model, ModelRequest};
use ai_models_core::ThinkingLevel;
use json_http::StaticHeaderAuth;
use serde_json::json;

use crate::AnthropicModel;

use super::stream_support::client_for_buffered_bodies;

#[tokio::test]
async fn output_limits_use_catalog_defaults_ceilings_and_unknown_fallback() {
    for (catalog_id, provider_id, requested, expected) in [
        ("claude-opus-5-thinking-max", "claude-opus-5", None, 128_000),
        ("claude-sonnet-5", "claude-sonnet-5", Some(9_000), 9_000),
        ("custom-alias", "claude-opus-4-7", Some(200_000), 128_000),
        ("claude-haiku-4-5", "claude-haiku-4-5", None, 64_000),
        (
            "claude-haiku-4-5",
            "claude-haiku-4-5",
            Some(100_000),
            64_000,
        ),
        ("claude-sonnet-5", "custom-provider-id", None, 64_000),
        ("custom", "custom-provider-id", Some(100_000), 64_000),
        ("custom", "custom-provider-id", Some(8_000), 8_000),
    ] {
        let (client, requests) = client_for_buffered_bodies(vec![json!({
            "content": [{"type": "text", "text": "Done"}], "stop_reason": "end_turn"
        })]);
        let model = AnthropicModel::with_catalog_auth(
            client,
            catalog_id,
            provider_id,
            ThinkingLevel::Max,
            Arc::new(StaticHeaderAuth::new(Default::default())),
        );
        let mut request = ModelRequest {
            system_prompt: String::new(),
            messages: vec![ConversationMessage::user("hello")],
            tools: Vec::new(),
            response_schema: None,
            controls: Default::default(),
        };
        request.controls.generation.max_output_tokens = requested;
        model.complete(&request).await.unwrap();
        let requests = requests.lock().unwrap();
        assert_eq!(
            requests[0].body.as_ref().unwrap()["max_tokens"],
            expected,
            "{catalog_id}/{provider_id}, requested {requested:?}"
        );
    }
}
