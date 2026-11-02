//! Provider configuration at the Anthropic streaming boundary.

use std::{sync::Arc, time::Duration};

use ai_interface::{
    ConversationMessage, ConversationPart, ConversationRole, ErrorKind, Model, ModelRequest,
    ProviderState,
};
use ai_models_core::ProviderConfig;
use json_http::TransportBackedJsonHttpClient;
use serde_json::json;
use unimock::Unimock;

use crate::{AnthropicModel, PROVIDER_ID};

use super::stream_support::{client_for_buffered_bodies, custom_spec};

#[tokio::test]
async fn config_stream_preserves_url_prefix_headers_and_timeout_precedence() {
    for (base, idle, total, call, expected_url, expected_total) in [
        (
            None,
            None,
            None,
            None,
            "https://api.anthropic.com/v1/messages",
            3600,
        ),
        (
            Some("https://proxy.test/team/"),
            Some(7),
            Some(31),
            None,
            "https://proxy.test/team/v1/messages",
            31,
        ),
        (
            Some("https://proxy.test/team"),
            Some(7),
            Some(31),
            Some(11),
            "https://proxy.test/team/v1/messages",
            11,
        ),
        (
            Some("https://proxy.test/team"),
            Some(7),
            Some(31),
            Some(61),
            "https://proxy.test/team/v1/messages",
            61,
        ),
    ] {
        let (http, requests) =
            client_for_buffered_bodies(vec![json!({"stop_reason":"end_turn", "content":[]})]);
        let mut config = ProviderConfig::new(PROVIDER_ID, "config-key")
            .with_header("X-Tenant", "tenant-value")
            .with_header("X-API-KEY", "wrong-key")
            .with_header("ANTHROPIC-VERSION", "wrong-version")
            .with_poll_interval(Duration::from_secs(1));
        config.base_url = base.map(str::to_owned);
        config.timeouts.idle_timeout = idle.map(Duration::from_secs);
        config.timeouts.total_timeout = total.map(Duration::from_secs);
        let model = AnthropicModel::from_config(http, custom_spec("claude-configured"), config);
        let mut request = request();
        request.controls.execution.total_timeout = call.map(Duration::from_secs);
        model.complete(&request).await.unwrap();

        let requests = requests.lock().unwrap();
        assert_eq!(requests.len(), 1);
        let request = &requests[0];
        assert_eq!(request.url, expected_url);
        assert_eq!(request.timeout, Duration::from_secs(expected_total));
        assert_eq!(
            request.idle_timeout,
            Some(Duration::from_secs(idle.unwrap_or(120)))
        );
        assert_eq!(
            request.headers.get("X-Tenant").map(String::as_str),
            Some("tenant-value")
        );
        assert_eq!(header_values(request, "x-api-key"), ["config-key"]);
        assert_eq!(header_values(request, "anthropic-version"), ["2023-06-01"]);
        let body: serde_json::Value =
            serde_json::from_slice(request.body.as_ref().unwrap().as_json_bytes().unwrap())
                .unwrap();
        assert_eq!(body["model"], "claude-configured");
    }
}

#[tokio::test]
async fn config_explicit_key_redacts_local_state_errors_before_auth() {
    let client = Arc::new(TransportBackedJsonHttpClient::new(Arc::new(Unimock::new(
        (),
    ))));
    let model = AnthropicModel::from_config(
        client,
        custom_spec("claude-configured"),
        ProviderConfig::new(PROVIDER_ID, "42"),
    );
    let mut request = request();
    request.messages = vec![ConversationMessage::new(
        ConversationRole::Assistant,
        vec![
            ConversationPart::text("saved").with_provider_state(ProviderState {
                provider: PROVIDER_ID,
                model: "claude-configured".to_owned(),
                version: 42,
                payload: Default::default(),
            }),
        ],
    )];
    let error = model.complete(&request).await.unwrap_err();
    assert_eq!(error.core().kind, ErrorKind::InvalidRequest);
    assert_eq!(error.core().provider, "anthropic");
    assert_eq!(error.core().model_id, "claude-configured");
    assert_eq!(
        error.core().message,
        "[ai_models_anthropic/replay] unsupported provider state version [redacted]"
    );
    assert!(!error.to_string().contains("42"));
    assert!(!format!("{error:?}").contains("42"));
}

pub(super) fn request() -> ModelRequest {
    ModelRequest {
        system_prompt: String::new(),
        messages: vec![ConversationMessage::user("hello")],
        tools: Vec::new(),
        response_schema: None,
        controls: Default::default(),
    }
}

pub(super) fn header_values<'a>(
    request: &'a json_http::JsonHttpRequest,
    header: &str,
) -> Vec<&'a str> {
    request
        .headers
        .iter()
        .filter(|(name, _)| name.eq_ignore_ascii_case(header))
        .map(|(_, value)| value.as_str())
        .collect()
}
