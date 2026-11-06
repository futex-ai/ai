//! Configured Responses requests and per-call timeout precedence.

use std::time::Duration;

use ai_interface::{ConversationMessage, Model, ModelRequest};
use ai_models_core::ProviderConfig;
use serde_json::json;

use crate::openai::stream_support::{client_for_buffered_bodies, custom_spec};
use crate::{OpenAiModel, PROVIDER_ID};

use super::support::{assert_headers, config, rotating_auth};

#[tokio::test]
async fn config_stream_uses_base_headers_auth_and_timeout_precedence() {
    for (base, idle, total, call, expected_url, expected_total) in [
        (
            None,
            None,
            None,
            None,
            "https://api.openai.com/v1/responses",
            3600,
        ),
        (
            Some("https://proxy.test/team/"),
            Some(7),
            Some(31),
            None,
            "https://proxy.test/team/v1/responses",
            31,
        ),
        (
            Some("https://proxy.test/team"),
            Some(7),
            Some(31),
            Some(11),
            "https://proxy.test/team/v1/responses",
            11,
        ),
        (
            Some("https://proxy.test/team"),
            Some(7),
            Some(31),
            Some(61),
            "https://proxy.test/team/v1/responses",
            61,
        ),
    ] {
        let (http, requests) =
            client_for_buffered_bodies(vec![json!({"status":"completed","output":[]})]);
        let mut config = config(base).with_poll_interval(Duration::from_secs(1));
        config.timeouts.idle_timeout = idle.map(Duration::from_secs);
        config.timeouts.total_timeout = total.map(Duration::from_secs);
        let model = OpenAiModel::from_config(http, custom_spec("configured-model"), config);
        let mut request = request();
        request.controls.execution.total_timeout = call.map(Duration::from_secs);
        model.complete(&request).await.unwrap();

        let requests = requests.lock().unwrap();
        assert_eq!(requests.len(), 1);
        assert_eq!(requests[0].url, expected_url);
        assert_eq!(requests[0].timeout, Duration::from_secs(expected_total));
        assert_eq!(
            requests[0].idle_timeout,
            Some(Duration::from_secs(idle.unwrap_or(120)))
        );
        assert_headers(&requests[0], "Bearer config-key");
    }
}

#[tokio::test]
async fn config_stream_applies_auth_hooks_for_each_call() {
    let (http, requests) = client_for_buffered_bodies(vec![
        json!({"status":"completed","output":[]}),
        json!({"status":"completed","output":[]}),
    ]);
    let config = ProviderConfig::with_auth(PROVIDER_ID, rotating_auth())
        .with_header("AUTHORIZATION", "wrong")
        .with_header("X-Tenant", "tenant-value");
    let model = OpenAiModel::from_config(http, custom_spec("configured-model"), config);
    model.complete(&request()).await.unwrap();
    model.complete(&request()).await.unwrap();
    let requests = requests.lock().unwrap();
    assert_eq!(requests.len(), 2);
    assert_headers(&requests[0], "Bearer fresh-1");
    assert_headers(&requests[1], "Bearer fresh-2");
}

fn request() -> ModelRequest {
    ModelRequest {
        system_prompt: String::new(),
        messages: vec![ConversationMessage::user("hello")],
        tools: Vec::new(),
        response_schema: None,
        controls: Default::default(),
    }
}
