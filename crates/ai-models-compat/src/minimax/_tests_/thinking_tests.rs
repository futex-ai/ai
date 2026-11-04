//! MiniMax thinking-control tests.

use std::sync::Arc;

use ai_interface::{Effort, Model, ModelRequest};
use json_http::{JsonHttpResponse, StaticHeaderAuth};
use serde_json::json;

use crate::minimax::{MINIMAX_M2_7, MINIMAX_M2_7_HIGHSPEED, MINIMAX_M3, known_models};

use super::{MiniMaxModel, support::recording_http_client};

#[tokio::test]
async fn maps_disabled_and_enabled_thinking_controls() {
    let cases = [
        (Effort::Disabled, Effort::Disabled, "disabled"),
        (Effort::Low, Effort::Disabled, "disabled"),
        (Effort::Medium, Effort::Medium, "adaptive"),
        (Effort::High, Effort::Medium, "adaptive"),
        (Effort::ExtraHigh, Effort::Medium, "adaptive"),
        (Effort::Max, Effort::Medium, "adaptive"),
    ];

    for (requested, effective, expected_control) in cases {
        let (http_client, requests) = recording_http_client([stopped_response()]);
        let model = MiniMaxModel::with_catalog_auth(
            http_client,
            MINIMAX_M3,
            Arc::new(StaticHeaderAuth::bearer_token("minimax-key")),
        );
        let mut request = simple_request();
        request.controls.generation.effort = Some(requested);
        let response = model
            .complete(&request)
            .await
            .expect("thinking response should parse");
        let requests = requests
            .lock()
            .expect("requests lock should not be poisoned");
        let body = requests[0]
            .body
            .as_ref()
            .and_then(|body| body.as_json())
            .expect("JSON body should be present");

        assert_eq!(body["thinking"]["type"], expected_control);
        assert_eq!(body["reasoning_split"], true);
        assert_eq!(
            response.effort().map(Effort::as_str),
            Some(effective.as_str())
        );
    }
}

#[test]
fn m2_7_catalog_entries_always_enable_thinking() {
    let models = known_models();
    for id in [MINIMAX_M2_7, MINIMAX_M2_7_HIGHSPEED] {
        let model = models
            .iter()
            .find(|model| model.provider_model_id == id.provider_model_id)
            .expect("M2.7 model should exist");
        assert!(model.default_effort.is_some_and(Effort::is_enabled));
    }
}

fn simple_request() -> ModelRequest {
    ModelRequest {
        system_prompt: "system".to_owned(),
        messages: Vec::new(),
        tools: Vec::new(),
        response_schema: None,
        controls: Default::default(),
    }
}

fn stopped_response() -> JsonHttpResponse<serde_json::Value> {
    JsonHttpResponse {
        status: 200,
        body: json!({
            "choices": [{
                "finish_reason": "stop",
                "message": {"content": "Done"}
            }]
        }),
    }
}
