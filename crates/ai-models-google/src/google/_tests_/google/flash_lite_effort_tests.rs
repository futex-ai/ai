//! Flash-Lite effort levels use explicit native controls and truthful metadata.

use std::sync::Arc;

use ai_interface::{Effort, Model, ModelControl, ModelError, ModelSpec};
use json_http::TransportBackedJsonHttpClient;
use serde_json::json;
use unimock::Unimock;

use crate::{CUSTOM_DEFAULTS, GEMINI_3_5_FLASH_LITE, GEMINI_3_6_FLASH, GoogleModel, PROVIDER_ID};

use super::{
    simple_request,
    stream_support::{event, recording_streaming_client},
};

#[tokio::test]
async fn flash_lite_efforts_reach_native_levels_and_lower_stronger_requests() {
    for (requested, resolved, native) in [
        (Effort::Low, Effort::Low, "minimal"),
        (Effort::Medium, Effort::Medium, "medium"),
        (Effort::High, Effort::High, "high"),
        (Effort::ExtraHigh, Effort::High, "high"),
        (Effort::Max, Effort::High, "high"),
    ] {
        let (http, requests) = recording_streaming_client(vec![vec![event(response())]]);
        let model = GoogleModel::new(http, GEMINI_3_5_FLASH_LITE, "key");
        let mut request = simple_request();
        request.controls.generation.effort = Some(requested);

        let response = model.complete(&request).await;
        let requests = requests.lock().unwrap();
        assert_eq!(requests.len(), 1, "{requested:?}: {response:?}");
        let body = requests[0].body.as_ref().unwrap();
        assert_eq!(
            body["generationConfig"]["thinkingConfig"],
            json!({"thinkingLevel": native}),
            "{requested:?}"
        );
        assert_eq!(response.unwrap().effort(), Some(resolved), "{requested:?}");
    }
}

#[tokio::test]
async fn flash_lite_default_sends_minimal_and_reports_low() {
    let (http, requests) = recording_streaming_client(vec![vec![event(response())]]);
    let model = GoogleModel::new(http, GEMINI_3_5_FLASH_LITE, "key");

    let response = model.complete(&simple_request()).await.unwrap();
    assert_eq!(response.effort(), Some(Effort::Low));
    assert_eq!(
        requests.lock().unwrap()[0].body.as_ref().unwrap()["generationConfig"]["thinkingConfig"],
        json!({"thinkingLevel": "minimal"})
    );
}

#[tokio::test]
async fn known_gemini_custom_specs_cannot_advertise_an_off_control() {
    for model_id in [
        GEMINI_3_5_FLASH_LITE.provider_model_id.as_ref(),
        GEMINI_3_6_FLASH.provider_model_id.as_ref(),
    ] {
        let http = Arc::new(TransportBackedJsonHttpClient::new(Arc::new(Unimock::new(
            (),
        ))));
        let spec = ModelSpec::custom(PROVIDER_ID, model_id.to_owned(), &CUSTOM_DEFAULTS);
        let model = GoogleModel::new(http, spec, "key");
        let mut request = simple_request();
        request.controls.generation.effort = Some(Effort::Disabled);

        assert!(matches!(model.complete(&request).await,
            Err(ModelError::UnsupportedControl { provider, model_id: rejected, control: ModelControl::Effort })
            if provider == "google" && rejected == model_id
        ));
    }
}

fn response() -> serde_json::Value {
    json!({"candidates": [{
        "content": {"parts": [{"text": "Done"}]}, "finishReason": "STOP"
    }]})
}
