//! DeepSeek model construction tests.

use std::sync::Arc;

use ai_interface::Model;
use ai_models_core::ThinkingLevel;
use json_http::StaticHeaderAuth;

use crate::{
    DEEPSEEK_V4_FLASH, DEEPSEEK_V4_FLASH_THINKING_DISABLED, DEEPSEEK_V4_FLASH_THINKING_MAX,
    DEEPSEEK_V4_PRO, DEEPSEEK_V4_PRO_THINKING_DISABLED, DEEPSEEK_V4_PRO_THINKING_MAX,
    DeepSeekConfigurationError,
};

use super::{
    DeepSeekModel,
    test_support::{
        recording_http_client, simple_request, successful_response, unused_http_client,
    },
};

#[tokio::test]
async fn default_constructor_selects_pro_with_high_thinking() {
    let (http_client, _) = recording_http_client(successful_response(Some("Done")));
    let response = DeepSeekModel::new(http_client, "deepseek-key")
        .complete(&simple_request())
        .await
        .expect("DeepSeek response should parse");

    assert_eq!(response.model_id, DEEPSEEK_V4_PRO);
    assert_eq!(response.catalog_model_id.as_deref(), Some(DEEPSEEK_V4_PRO));
    assert_eq!(response.thinking_level.as_deref(), Some("high"));
}

#[tokio::test]
async fn accepts_every_supported_provider_and_thinking_combination() {
    let cases = [
        (DEEPSEEK_V4_PRO, DEEPSEEK_V4_PRO, ThinkingLevel::High),
        (
            DEEPSEEK_V4_PRO_THINKING_MAX,
            DEEPSEEK_V4_PRO,
            ThinkingLevel::Max,
        ),
        (
            DEEPSEEK_V4_PRO_THINKING_DISABLED,
            DEEPSEEK_V4_PRO,
            ThinkingLevel::Disabled,
        ),
        (DEEPSEEK_V4_FLASH, DEEPSEEK_V4_FLASH, ThinkingLevel::High),
        (
            DEEPSEEK_V4_FLASH_THINKING_MAX,
            DEEPSEEK_V4_FLASH,
            ThinkingLevel::Max,
        ),
        (
            DEEPSEEK_V4_FLASH_THINKING_DISABLED,
            DEEPSEEK_V4_FLASH,
            ThinkingLevel::Disabled,
        ),
    ];

    for (catalog_id, provider_id, thinking_level) in cases {
        let (http_client, requests) = recording_http_client(successful_response(Some("Done")));
        let model = DeepSeekModel::with_catalog_auth(
            http_client,
            catalog_id,
            provider_id,
            thinking_level,
            Arc::new(StaticHeaderAuth::default()),
        )
        .expect("supported configuration should build");
        let response = model
            .complete(&simple_request())
            .await
            .expect("DeepSeek response should parse");

        assert_eq!(response.model_id, provider_id);
        assert_eq!(response.catalog_model_id.as_deref(), Some(catalog_id));
        assert_eq!(
            response.thinking_level.as_deref(),
            Some(thinking_level.as_str())
        );
        assert_eq!(
            requests
                .lock()
                .expect("requests lock should not be poisoned")[0]
                .body
                .as_ref()
                .expect("request body")["model"],
            provider_id
        );
    }
}

#[test]
fn rejects_unknown_provider_model_ids() {
    let result = DeepSeekModel::with_catalog_auth(
        unused_http_client(),
        "retired",
        "deepseek-chat",
        ThinkingLevel::High,
        Arc::new(StaticHeaderAuth::default()),
    );

    assert!(matches!(
        result,
        Err(DeepSeekConfigurationError::UnsupportedProviderModel {
            provider_model_id
        }) if provider_model_id == "deepseek-chat"
    ));
}

#[tokio::test]
async fn downgrades_unsupported_thinking_levels() {
    for (requested, effective) in [
        (ThinkingLevel::Low, ThinkingLevel::Disabled),
        (ThinkingLevel::Medium, ThinkingLevel::Disabled),
        (ThinkingLevel::ExtraHigh, ThinkingLevel::High),
    ] {
        let (http_client, _) = recording_http_client(successful_response(Some("Done")));
        let model = DeepSeekModel::with_catalog_auth(
            http_client,
            DEEPSEEK_V4_PRO,
            DEEPSEEK_V4_PRO,
            requested,
            Arc::new(StaticHeaderAuth::default()),
        )
        .expect("unsupported thinking level should downgrade");
        let response = model
            .complete(&simple_request())
            .await
            .expect("DeepSeek response should parse");

        assert_eq!(response.thinking_level.as_deref(), Some(effective.as_str()));
    }
}
