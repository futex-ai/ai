//! Qwen model construction tests.

use std::{collections::BTreeMap, sync::Arc};

use ai_interface::{Model, ProviderKind};
use ai_models_core::ThinkingLevel;
use json_http::StaticHeaderAuth;

use crate::{QWEN_3_7_MAX, QWEN_3_7_PLUS, QwenConfigurationError};

use super::{
    QwenModel,
    test_support::{
        recording_http_client, simple_request, successful_response, unused_http_client,
    },
};

#[test]
fn rejects_unsupported_provider_model_id() {
    let result = QwenModel::with_catalog_auth(
        unused_http_client(),
        QWEN_3_7_PLUS,
        "qwen-preview",
        ThinkingLevel::High,
        Arc::new(StaticHeaderAuth::default()),
    );

    assert!(matches!(
        result,
        Err(QwenConfigurationError::UnsupportedProviderModel { provider_model_id })
            if provider_model_id == "qwen-preview"
    ));
}

#[tokio::test]
async fn downgrades_unsupported_thinking_levels() {
    for (requested, effective, enabled) in [
        (ThinkingLevel::Low, ThinkingLevel::Disabled, false),
        (ThinkingLevel::Medium, ThinkingLevel::Disabled, false),
        (ThinkingLevel::ExtraHigh, ThinkingLevel::High, true),
        (ThinkingLevel::Max, ThinkingLevel::High, true),
    ] {
        let (http_client, requests) = recording_http_client(successful_response(Some("Done")));
        let model = QwenModel::with_catalog_auth(
            http_client,
            QWEN_3_7_MAX,
            QWEN_3_7_MAX,
            requested,
            Arc::new(StaticHeaderAuth::default()),
        )
        .expect("unsupported thinking level should downgrade");
        let response = model
            .complete(&simple_request())
            .await
            .expect("Qwen response should parse");
        let requests = requests
            .lock()
            .expect("requests lock should not be poisoned");
        let body = requests[0]
            .body
            .as_ref()
            .and_then(|body| body.as_json())
            .expect("JSON request body");

        assert_eq!(body["enable_thinking"], enabled);
        assert_eq!(response.thinking_level.as_deref(), Some(effective.as_str()));
    }
}

#[tokio::test]
async fn new_applies_bearer_auth() {
    let (http_client, requests) = recording_http_client(successful_response(Some("Done")));
    let model = QwenModel::new(http_client, "qwen-secret");

    model
        .complete(&simple_request())
        .await
        .expect("Qwen response should parse");
    let requests = requests
        .lock()
        .expect("requests lock should not be poisoned");

    assert_eq!(
        requests[0].headers.get("Authorization").map(String::as_str),
        Some("Bearer qwen-secret")
    );
}

#[tokio::test]
async fn default_uses_plus_high_thinking_endpoint_and_injected_auth() {
    let (http_client, requests) = recording_http_client(successful_response(Some("Done")));
    let auth = Arc::new(StaticHeaderAuth::new(BTreeMap::from([(
        "X-Test-Auth".to_owned(),
        "injected".to_owned(),
    )])));
    let model = QwenModel::with_auth(http_client, auth);

    let response = model
        .complete(&simple_request())
        .await
        .expect("Qwen response should parse");
    let requests = requests
        .lock()
        .expect("requests lock should not be poisoned");
    let body = requests[0]
        .body
        .as_ref()
        .and_then(|body| body.as_json())
        .expect("JSON request body");

    assert_eq!(response.provider, ProviderKind::Qwen.as_str());
    assert_eq!(response.model_id, QWEN_3_7_PLUS);
    assert_eq!(response.thinking_level.as_deref(), Some("high"));
    assert_eq!(body["model"], QWEN_3_7_PLUS);
    assert_eq!(body["enable_thinking"], true);
    assert_eq!(body["preserve_thinking"], true);
    assert_eq!(
        requests[0].headers.get("X-Test-Auth").map(String::as_str),
        Some("injected")
    );
    assert_eq!(
        requests[0].url,
        "https://dashscope-intl.aliyuncs.com/compatible-mode/v1/chat/completions"
    );
}
