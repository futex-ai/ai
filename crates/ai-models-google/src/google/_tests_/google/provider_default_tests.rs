//! Provider defaults do not claim that Gemini thinking is disabled.

use std::sync::Arc;

use ai_interface::{Effort, Model, ModelControl, ModelError, ModelSpec};
use json_http::TransportBackedJsonHttpClient;
use serde_json::json;
use unimock::Unimock;

use crate::{CUSTOM_DEFAULTS, GEMINI_3_5_FLASH_LITE, GoogleModel, PROVIDER_ID};

use super::{
    simple_request,
    stream_support::{event, recording_streaming_client},
};

#[tokio::test]
async fn provider_default_omits_thinking_and_reports_no_effort() {
    for spec in [
        GEMINI_3_5_FLASH_LITE,
        ModelSpec::custom(PROVIDER_ID, "gemini-custom", &CUSTOM_DEFAULTS),
    ] {
        let (http, requests) = recording_streaming_client(vec![vec![event(json!({
            "candidates": [{"content": {"parts": [{"text": "Done"}]}, "finishReason": "STOP"}]
        }))]]);
        let model = GoogleModel::new(http, spec, "key");
        let response = model.complete(&simple_request()).await.unwrap();
        assert_eq!(response.effort(), None);
        let requests = requests.lock().unwrap();
        assert_eq!(
            requests[0].body.as_ref().unwrap()["generationConfig"].get("thinkingConfig"),
            None
        );
    }
}

#[tokio::test]
async fn flash_lite_rejects_explicit_off_before_transport() {
    let http = Arc::new(TransportBackedJsonHttpClient::new(Arc::new(Unimock::new(
        (),
    ))));
    let model = GoogleModel::new(http, GEMINI_3_5_FLASH_LITE, "key");
    let mut request = simple_request();
    request.controls.generation.effort = Some(Effort::Disabled);
    let result = model.complete(&request).await;
    assert!(matches!(result,
        Err(ModelError::UnsupportedControl { provider, model_id, control: ModelControl::Effort })
        if provider == "google" && model_id == "gemini-3.5-flash-lite"
    ));
}

#[tokio::test]
async fn explicit_off_sends_zero_budget_for_custom_flash_2_5() {
    let (http, requests) = recording_streaming_client(vec![vec![event(json!({
        "candidates": [{"content": {"parts": [{"text": "Done"}]}, "finishReason": "STOP"}]
    }))]]);
    let spec = ModelSpec::custom(PROVIDER_ID, "gemini-2.5-flash", &CUSTOM_DEFAULTS);
    let model = GoogleModel::new(http, spec, "key");
    let mut request = simple_request();
    request.controls.generation.effort = Some(Effort::Disabled);
    let response = model.complete(&request).await.unwrap();
    assert_eq!(
        requests.lock().unwrap()[0].body.as_ref().unwrap()["generationConfig"]["thinkingConfig"],
        json!({"thinkingBudget": 0})
    );
    assert_eq!(response.effort(), Some(Effort::Disabled));
}
