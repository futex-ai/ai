//! DeepSeek spec construction and per-request effort tests.

use std::sync::Arc;

use ai_interface::{Effort, Model, ModelSpec};
use json_http::StaticHeaderAuth;

use super::{
    CUSTOM_DEFAULTS, DEEPSEEK_V4_FLASH, DEEPSEEK_V4_PRO, DeepSeekModel, PROVIDER_ID,
    test_support::{recording_http_client, simple_request, successful_response},
};

#[tokio::test]
async fn default_constructor_selects_pro_with_high_thinking() {
    let (http_client, _) = recording_http_client(successful_response(Some("Done")));
    let response = DeepSeekModel::new(http_client, "deepseek-key")
        .complete(&simple_request())
        .await
        .expect("DeepSeek response should parse");
    assert_eq!(response.model_id, "deepseek-v4-pro");
    assert_eq!(
        response.catalog_model_id.as_deref(),
        Some("deepseek/deepseek-v4-pro")
    );
    assert_eq!(response.effort().map(Effort::as_str), Some("high"));
}

#[tokio::test]
async fn accepts_every_supported_provider_and_request_effort_combination() {
    for spec in [DEEPSEEK_V4_PRO, DEEPSEEK_V4_FLASH] {
        for effort in [Effort::High, Effort::Max, Effort::Disabled] {
            let (http_client, requests) = recording_http_client(successful_response(Some("Done")));
            let model = DeepSeekModel::with_catalog_auth(
                http_client,
                spec.clone(),
                Arc::new(StaticHeaderAuth::default()),
            );
            let mut request = simple_request();
            request.controls.generation.effort = Some(effort);
            let response = model
                .complete(&request)
                .await
                .expect("DeepSeek response should parse");
            assert_eq!(response.model_id, spec.provider_model_id);
            assert_eq!(
                response.catalog_model_id.as_deref(),
                Some(spec.text_id().as_str())
            );
            assert_eq!(response.effort().map(Effort::as_str), Some(effort.as_str()));
            assert_eq!(
                requests.lock().expect("requests lock")[0]
                    .body
                    .as_ref()
                    .expect("body")["model"],
                spec.provider_model_id.as_ref()
            );
        }
    }
}

#[tokio::test]
async fn custom_provider_model_reaches_transport_without_catalog_validation() {
    let (http_client, requests) = recording_http_client(successful_response(Some("Done")));
    let spec = ModelSpec::custom(PROVIDER_ID, "deepseek-custom", &CUSTOM_DEFAULTS);
    let model =
        DeepSeekModel::with_catalog_auth(http_client, spec, Arc::new(StaticHeaderAuth::default()));
    let response = model
        .complete(&simple_request())
        .await
        .expect("custom response");
    assert_eq!(response.model_id, "deepseek-custom");
    assert_eq!(
        response.catalog_model_id.as_deref(),
        Some("deepseek/deepseek-custom")
    );
    assert_eq!(
        requests.lock().expect("requests lock")[0]
            .body
            .as_ref()
            .expect("body")["model"],
        "deepseek-custom"
    );
}

#[tokio::test]
async fn downgrades_unsupported_request_efforts() {
    for (requested, effective) in [
        (Effort::Low, Effort::Disabled),
        (Effort::Medium, Effort::Disabled),
        (Effort::ExtraHigh, Effort::High),
    ] {
        let (http_client, _) = recording_http_client(successful_response(Some("Done")));
        let model = DeepSeekModel::with_catalog_auth(
            http_client,
            DEEPSEEK_V4_PRO,
            Arc::new(StaticHeaderAuth::default()),
        );
        let mut request = simple_request();
        request.controls.generation.effort = Some(requested);
        let response = model
            .complete(&request)
            .await
            .expect("DeepSeek response should parse");
        assert_eq!(
            response.effort().map(Effort::as_str),
            Some(effective.as_str())
        );
    }
}
