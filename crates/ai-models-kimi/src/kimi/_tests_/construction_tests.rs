//! Kimi model construction tests.

use std::{collections::BTreeMap, sync::Arc};

use ai_interface::{Model, ProviderKind};
use ai_models_core::ThinkingLevel;
use json_http::StaticHeaderAuth;

use crate::{KIMI_K3, KimiConfigurationError};

use super::{
    KimiModel,
    test_support::{
        recording_http_client, simple_request, successful_response, unused_http_client,
    },
};

#[test]
fn rejects_unsupported_provider_model_id() {
    let result = KimiModel::with_catalog_auth(
        unused_http_client(),
        KIMI_K3,
        "moonshot-v1-128k",
        ThinkingLevel::Max,
        Arc::new(StaticHeaderAuth::default()),
    );

    assert!(matches!(
        result,
        Err(KimiConfigurationError::UnsupportedProviderModel {
            provider_model_id
        }) if provider_model_id == "moonshot-v1-128k"
    ));
}

#[test]
fn rejects_thinking_below_the_lowest_supported_level() {
    let result = KimiModel::with_catalog_auth(
        unused_http_client(),
        KIMI_K3,
        KIMI_K3,
        ThinkingLevel::Disabled,
        Arc::new(StaticHeaderAuth::default()),
    );

    assert!(matches!(
        result,
        Err(KimiConfigurationError::UnsupportedThinkingLevel {
            thinking_level: "disabled"
        })
    ));
}

#[tokio::test]
async fn downgrades_unsupported_thinking_levels() {
    for (requested, effective) in [
        (ThinkingLevel::Medium, ThinkingLevel::Low),
        (ThinkingLevel::ExtraHigh, ThinkingLevel::High),
    ] {
        let (http_client, _) = recording_http_client(successful_response(Some("Done")));
        let model = KimiModel::with_catalog_auth(
            http_client,
            KIMI_K3,
            KIMI_K3,
            requested,
            Arc::new(StaticHeaderAuth::default()),
        )
        .expect("unsupported thinking level should downgrade");
        let response = model
            .complete(&simple_request())
            .await
            .expect("Kimi response should parse");

        assert_eq!(response.thinking_level.as_deref(), Some(effective.as_str()));
    }
}

#[tokio::test]
async fn uses_injected_http_client_and_auth_hook() {
    let (http_client, requests) = recording_http_client(successful_response(Some("Done")));
    let auth = Arc::new(StaticHeaderAuth::new(BTreeMap::from([(
        "X-Test-Auth".to_owned(),
        "injected".to_owned(),
    )])));
    let model = KimiModel::with_auth(http_client, auth);

    let response = model
        .complete(&simple_request())
        .await
        .expect("Kimi response should parse");

    let requests = requests
        .lock()
        .expect("requests lock should not be poisoned");
    assert_eq!(
        requests[0].headers.get("X-Test-Auth").map(String::as_str),
        Some("injected")
    );
    assert_eq!(response.provider, ProviderKind::Kimi.as_str());
}
