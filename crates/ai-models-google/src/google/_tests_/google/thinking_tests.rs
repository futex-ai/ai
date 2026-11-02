//! Google thinking request tests.

use std::{collections::BTreeMap, sync::Arc};

use ai_interface::{Effort, Model};
use json_http::{JsonHttpResponse, StaticHeaderAuth};
use serde_json::json;

use crate::{GEMINI_2_5_PRO, GEMINI_3_6_FLASH};

use super::{GoogleModel, recording_http_client, simple_request, stream_support::custom_spec};

#[tokio::test]
async fn maps_gemini_2_5_thinking_to_token_budget() {
    let (http_client, requests) = recording_http_client(google_thinking_response());
    let model =
        GoogleModel::with_catalog_auth(http_client, custom_spec(GEMINI_2_5_PRO), google_auth());

    let mut request = simple_request();
    request.controls.generation.effort = Some(Effort::Max);
    let response = model
        .complete(&request)
        .await
        .expect("Google thinking response should parse");

    let requests = requests
        .lock()
        .expect("requests lock should not be poisoned");
    let body = requests[0].body.as_ref().expect("body present");
    assert_eq!(
        body["generationConfig"]["thinkingConfig"]["thinkingBudget"],
        32768
    );
    assert!(
        body["generationConfig"]["thinkingConfig"]
            .get("thinkingLevel")
            .is_none()
    );
    assert_eq!(
        response.catalog_model_id.as_deref(),
        Some("google/gemini-2.5-pro")
    );
    assert_thinking_response(&response, "max", "gemini-2.5-pro");
}

#[tokio::test]
async fn maps_gemini_3_thinking_to_named_level() {
    let (http_client, requests) = recording_http_client(google_thinking_response());
    let model = GoogleModel::with_catalog_auth(http_client, GEMINI_3_6_FLASH, google_auth());

    let mut request = simple_request();
    request.controls.generation.effort = Some(Effort::High);
    let response = model
        .complete(&request)
        .await
        .expect("Gemini 3 thinking response should parse");

    let requests = requests
        .lock()
        .expect("requests lock should not be poisoned");
    let body = requests[0].body.as_ref().expect("body present");
    assert_eq!(
        body["generationConfig"]["thinkingConfig"]["thinkingLevel"],
        "high"
    );
    assert!(
        body["generationConfig"]["thinkingConfig"]
            .get("thinkingBudget")
            .is_none()
    );
    assert_eq!(
        response.catalog_model_id.as_deref(),
        Some("google/gemini-3.6-flash")
    );
    assert_thinking_response(
        &response,
        "high",
        GEMINI_3_6_FLASH.provider_model_id.as_ref(),
    );
}

#[tokio::test]
async fn downgrades_gemini_3_max_to_high() {
    let (http_client, requests) = recording_http_client(google_thinking_response());
    let model = GoogleModel::with_catalog_auth(http_client, GEMINI_3_6_FLASH, google_auth());

    let mut request = simple_request();
    request.controls.generation.effort = Some(Effort::Max);
    let response = model
        .complete(&request)
        .await
        .expect("Gemini 3 downgraded response should parse");

    let requests = requests
        .lock()
        .expect("requests lock should not be poisoned");
    let body = requests[0].body.as_ref().expect("body present");
    assert_eq!(
        body["generationConfig"]["thinkingConfig"]["thinkingLevel"],
        "high"
    );
    assert_thinking_response(
        &response,
        "high",
        GEMINI_3_6_FLASH.provider_model_id.as_ref(),
    );
}

fn google_thinking_response() -> JsonHttpResponse<serde_json::Value> {
    JsonHttpResponse {
        status: 200,
        body: json!({
            "candidates": [{
                "finishReason": "STOP",
                "content": {
                    "parts": [
                        { "text": "hidden provider thought", "thought": true },
                        { "text": "Done" }
                    ]
                }
            }],
            "usageMetadata": {
                "promptTokenCount": 12,
                "candidatesTokenCount": 6,
                "cachedContentTokenCount": 4,
                "thoughtsTokenCount": 5,
                "totalTokenCount": 23
            }
        }),
    }
}

fn google_auth() -> Arc<StaticHeaderAuth> {
    Arc::new(StaticHeaderAuth::new(BTreeMap::from([(
        "x-goog-api-key".to_owned(),
        "google-key".to_owned(),
    )])))
}

fn assert_thinking_response(
    response: &ai_interface::ModelResponse,
    thinking_level: &str,
    model_id: &str,
) {
    assert_eq!(response.effort().map(Effort::as_str), Some(thinking_level));
    assert_eq!(response.model_id, model_id);
    assert_eq!(response.assistant_message, "Done");
    assert!(
        !response
            .assistant_message
            .contains("hidden provider thought")
    );
    assert_eq!(response.usage.input_tokens, 8);
    assert_eq!(response.usage.output_tokens, 6);
    assert_eq!(response.usage.total_tokens, 23);
    assert_eq!(response.usage.cached_input_tokens, 4);
    assert_eq!(response.usage.reasoning_tokens, 5);
}
