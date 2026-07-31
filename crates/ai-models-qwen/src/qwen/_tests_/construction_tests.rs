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

#[test]
fn rejects_every_unsupported_thinking_level() {
    for thinking_level in [
        ThinkingLevel::Low,
        ThinkingLevel::Medium,
        ThinkingLevel::ExtraHigh,
        ThinkingLevel::Max,
    ] {
        let result = QwenModel::with_catalog_auth(
            unused_http_client(),
            QWEN_3_7_MAX,
            QWEN_3_7_MAX,
            thinking_level,
            Arc::new(StaticHeaderAuth::default()),
        );

        assert!(matches!(
            result,
            Err(QwenConfigurationError::UnsupportedThinkingLevel {
                thinking_level: actual
            }) if actual == thinking_level.as_str()
        ));
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
