//! Request effort changes on each call without rebuilding the Google model.

use ai_interface::{Effort, Model};
use json_http::JsonHttpBody;
use serde_json::json;

use crate::GEMINI_3_6_FLASH;

use super::{
    GoogleModel, simple_request,
    stream_support::{event, recording_streaming_client},
};

#[tokio::test]
async fn request_effort_overrides_default_and_caps_output_for_each_call() {
    let body = json!({"candidates": [{
        "content": {"parts": [{"text": "Done"}]}, "finishReason": "STOP"
    }]});
    let (client, requests) =
        recording_streaming_client(vec![vec![event(body.clone())], vec![event(body)]]);
    let model = GoogleModel::new(client, GEMINI_3_6_FLASH, "key");
    let mut request = simple_request();
    request.controls.generation.effort = Some(Effort::High);
    request.controls.generation.max_output_tokens = Some(100_000);

    let explicit = model.complete(&request).await.unwrap();
    assert_eq!(explicit.effort().map(Effort::as_str), Some("high"));

    request.controls.generation.effort = None;
    let default = model.complete(&request).await.unwrap();
    assert_eq!(default.effort().map(Effort::as_str), Some("medium"));

    let requests = requests.lock().unwrap();
    let explicit = &requests[0].body.as_ref().unwrap()["generationConfig"];
    let default = &requests[1].body.as_ref().unwrap()["generationConfig"];
    assert_eq!(explicit["thinkingConfig"]["thinkingLevel"], "high");
    assert_eq!(default["thinkingConfig"]["thinkingLevel"], "medium");
    assert_eq!(explicit["maxOutputTokens"], 65_536);
}

#[tokio::test]
async fn flash_3_6_low_matches_legacy_wire_request() {
    let body = json!({"candidates": [{
        "content": {"parts": [{"text": "Done"}]}, "finishReason": "STOP"
    }]});
    let (client, requests) = recording_streaming_client(vec![vec![event(body)]]);
    let model = GoogleModel::new(client, GEMINI_3_6_FLASH, "key");
    let mut request = simple_request();
    request.controls.generation.effort = Some(Effort::Low);
    request.controls.generation.max_output_tokens = Some(256);
    request.controls.generation.temperature = Some(0.4);
    request.controls.generation.top_p = Some(0.8);

    let response = model.complete(&request).await;
    let requests = requests.lock().unwrap();
    assert_eq!(
        requests
            .iter()
            .map(|request| request.body.clone())
            .collect::<Vec<_>>(),
        vec![Some(JsonHttpBody::Json(json!({
            "systemInstruction": {"parts": [{"text": "system"}]},
            "contents": [{"role": "user", "parts": [{"text": "hello"}]}],
            "generationConfig": {
                "thinkingConfig": {"thinkingLevel": "low"},
                "maxOutputTokens": 256,
                "temperature": 0.4,
                "topP": 0.8
            }
        })))],
        "gemini-3.6-flash/low: {response:?}"
    );
    assert_eq!(response.unwrap().effort(), Some(Effort::Low));
}
