//! Registry model factories forward config and retain capability-bearing specs.

use std::time::Duration;

use ai_interface::{ConversationMessage, ErrorKind, ModelControl, ModelRequest};
use ai_models_core::test_support::recording_streaming_client;
use serde_json::Value;

use crate::{ProviderConfig, Registry};

use super::{model_support::cases, support::unused_http};

#[tokio::test]
async fn model_factories_forward_replacement_config_and_keep_known_and_custom_specs() {
    for case in cases() {
        let (http, requests) = recording_streaming_client(vec![case.fixture(), case.fixture()]);
        let provider = case.known.provider.clone();
        let mut registry =
            Registry::new(http).with_provider(ProviderConfig::new(provider.clone(), "old-key"));
        registry.insert(
            ProviderConfig::new(provider, "registry-key")
                .with_base_url("https://proxy.test/team/")
                .with_header("X-Tenant", "tenant")
                .with_total_timeout(Duration::from_secs(31))
                .with_idle_timeout(Duration::from_secs(7)),
        );
        let mut custom = case.custom.clone();
        custom.max_output_tokens = 13;
        for (index, (spec, expected_output)) in
            [(&case.known, 100), (&custom, 13)].into_iter().enumerate()
        {
            let built = registry.model(spec);
            assert!(built.is_ok(), "{}", spec.text_id());
            let model = built.unwrap();
            let mut request = request();
            request.controls.generation.max_output_tokens = Some(100);

            let response = model.complete(&request).await.unwrap();

            assert_eq!(response.provider, spec.provider.as_str());
            assert_eq!(response.model_id, spec.provider_model_id);
            assert_eq!(
                response.catalog_model_id.as_deref(),
                Some(spec.text_id().as_str())
            );
            assert_eq!(response.thinking_level, Some(spec.default_effort));
            let requests = requests.lock().unwrap();
            let sent = &requests[index];
            let path = case.path.replace("{model}", &spec.provider_model_id);
            assert_eq!(sent.url, format!("https://proxy.test/team{path}"));
            assert_eq!(sent.timeout, Duration::from_secs(31));
            assert_eq!(sent.idle_timeout, Some(Duration::from_secs(7)));
            assert_eq!(
                sent.headers.get("X-Tenant").map(String::as_str),
                Some("tenant")
            );
            let expected_auth = if case.auth_header == "Authorization" {
                "Bearer registry-key"
            } else {
                "registry-key"
            };
            assert_eq!(
                sent.headers.get(case.auth_header).map(String::as_str),
                Some(expected_auth)
            );
            let body: Value =
                serde_json::from_slice(sent.body.as_ref().unwrap().as_json_bytes().unwrap())
                    .unwrap();
            assert_eq!(
                body.pointer(case.output_pointer),
                Some(&Value::from(expected_output))
            );
        }
        assert_eq!(requests.lock().unwrap().len(), 2);
    }
}

#[tokio::test]
async fn model_factories_apply_the_custom_spec_capability_check_before_transport() {
    for case in cases() {
        let mut spec = case.custom;
        spec.supports_stop_sequences = false;
        let registry = Registry::new(unused_http())
            .with_provider(ProviderConfig::new(spec.provider.clone(), "registry-key"));
        let built = registry.model(&spec);
        assert!(built.is_ok(), "{}", spec.text_id());
        let model = built.unwrap();
        let mut request = request();
        request.controls.generation.stop_sequences = vec!["stop".into()];

        let error = model.complete(&request).await.unwrap_err();

        assert_eq!(
            error.core().kind,
            ErrorKind::UnsupportedControl(ModelControl::StopSequences)
        );
        assert_eq!(error.core().provider, spec.provider.as_str());
        assert_eq!(error.core().model_id, "registry-unlisted");
    }
}

fn request() -> ModelRequest {
    ModelRequest {
        system_prompt: String::new(),
        messages: vec![ConversationMessage::user("Hello")],
        tools: Vec::new(),
        response_schema: None,
        controls: Default::default(),
    }
}
