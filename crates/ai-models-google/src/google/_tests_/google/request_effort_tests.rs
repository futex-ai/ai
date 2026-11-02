//! Request effort changes on each call without rebuilding the Google model.

use ai_interface::{Effort, Model};
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
