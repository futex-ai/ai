//! Qwen model construction tests.

use std::{collections::BTreeMap, sync::Arc};

use ai_interface::{Effort, Model, ModelSpec};
use json_http::StaticHeaderAuth;

use crate::qwen::{CUSTOM_DEFAULTS, PROVIDER_ID, QWEN_3_7_MAX};

use super::{
    QwenModel,
    test_support::{recording_http_client, simple_request, successful_response},
};

#[tokio::test]
async fn custom_provider_model_reaches_transport_without_catalog_validation() {
    let (http_client, requests) = recording_http_client(successful_response(Some("Done")));
    let spec = ModelSpec::custom(PROVIDER_ID, "qwen-custom", &CUSTOM_DEFAULTS);
    let model =
        QwenModel::with_catalog_auth(http_client, spec, Arc::new(StaticHeaderAuth::default()));
    let response = model
        .complete(&simple_request())
        .await
        .expect("custom response");
    assert_eq!(response.model_id, "qwen-custom");
    assert_eq!(
        requests.lock().expect("requests lock")[0]
            .body
            .as_ref()
            .expect("body")["model"],
        "qwen-custom"
    );
}

#[tokio::test]
async fn downgrades_unsupported_request_efforts() {
    for (requested, effective, enabled) in [
        (Effort::Low, Effort::Disabled, false),
        (Effort::Medium, Effort::Disabled, false),
        (Effort::ExtraHigh, Effort::High, true),
        (Effort::Max, Effort::High, true),
    ] {
        let (http_client, requests) = recording_http_client(successful_response(Some("Done")));
        let model = QwenModel::with_catalog_auth(
            http_client,
            QWEN_3_7_MAX,
            Arc::new(StaticHeaderAuth::default()),
        );
        let mut request = simple_request();
        request.controls.generation.effort = Some(requested);
        let response = model
            .complete(&request)
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
        assert_eq!(
            response.effort().map(Effort::as_str),
            Some(effective.as_str())
        );
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

    assert_eq!(response.provider, "qwen");
    assert_eq!(response.model_id, "qwen3.7-plus");
    assert_eq!(response.effort().map(Effort::as_str), Some("high"));
    assert_eq!(body["model"], "qwen3.7-plus");
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
