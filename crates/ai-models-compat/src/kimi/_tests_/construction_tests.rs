//! Kimi spec construction and per-request effort tests.

use std::{collections::BTreeMap, sync::Arc};

use ai_interface::{Effort, Model, ModelControl, ModelError, ModelSpec};
use json_http::StaticHeaderAuth;

use super::{
    CUSTOM_DEFAULTS, KIMI_K3, KimiModel, PROVIDER_ID,
    test_support::{
        recording_http_client, simple_request, successful_response, unused_http_client,
    },
};

#[tokio::test]
async fn custom_provider_model_reaches_transport_without_catalog_validation() {
    let (http_client, requests) = recording_http_client(successful_response(Some("Done")));
    let spec = ModelSpec::custom(PROVIDER_ID, "kimi-custom", &CUSTOM_DEFAULTS);
    let model =
        KimiModel::with_catalog_auth(http_client, spec, Arc::new(StaticHeaderAuth::default()));
    let response = model
        .complete(&simple_request())
        .await
        .expect("custom response");
    assert_eq!(response.model_id, "kimi-custom");
    assert_eq!(
        requests.lock().expect("requests lock")[0]
            .body
            .as_ref()
            .expect("body")["model"],
        "kimi-custom"
    );
}

#[tokio::test]
async fn rejects_request_effort_below_lowest_supported_before_transport() {
    let model = KimiModel::with_catalog_auth(
        unused_http_client(),
        KIMI_K3,
        Arc::new(StaticHeaderAuth::default()),
    );
    let mut request = simple_request();
    request.controls.generation.effort = Some(Effort::Disabled);
    let error = model
        .complete(&request)
        .await
        .expect_err("disabled effort must fail");
    assert!(
        matches!(error, ModelError::UnsupportedControl { provider, model_id, control: ModelControl::Effort } if provider == "kimi" && model_id == "kimi-k3")
    );
}

#[tokio::test]
async fn downgrades_unsupported_request_efforts() {
    for (requested, effective) in [
        (Effort::Medium, Effort::Low),
        (Effort::ExtraHigh, Effort::High),
    ] {
        let (http_client, _) = recording_http_client(successful_response(Some("Done")));
        let model = KimiModel::with_catalog_auth(
            http_client,
            KIMI_K3,
            Arc::new(StaticHeaderAuth::default()),
        );
        let mut request = simple_request();
        request.controls.generation.effort = Some(requested);
        let response = model
            .complete(&request)
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
    assert_eq!(
        requests.lock().expect("requests lock")[0]
            .headers
            .get("X-Test-Auth")
            .map(String::as_str),
        Some("injected")
    );
    assert_eq!(response.provider, "kimi");
}
