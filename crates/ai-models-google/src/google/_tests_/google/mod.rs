//! Tests for Google request mapping and response parsing.

use std::{
    collections::VecDeque,
    sync::{Arc, Mutex},
};

use ai_interface::{ConversationMessage, ModelRequest};
use json_http::{
    JsonHttpClient, JsonHttpRequest, JsonHttpResponse, JsonHttpTransportMock,
    TransportBackedJsonHttpClient,
};
use serde_json::json;
use unimock::{MockFn, Unimock, matching};

use super::GoogleModel;

mod finish_reason_tests;
mod multimodal_tests;
mod request_tests;
mod structured_finish_tests;

type RecordedRequests = Arc<Mutex<Vec<JsonHttpRequest>>>;

fn recording_http_client(
    response: JsonHttpResponse<serde_json::Value>,
) -> (Arc<dyn JsonHttpClient>, RecordedRequests) {
    let requests = Arc::new(Mutex::new(Vec::new()));
    let responses = Arc::new(Mutex::new(VecDeque::from([response])));
    let transport = Arc::new(Unimock::new(
        JsonHttpTransportMock::execute
            .each_call(matching!(_))
            .answers_arc({
                let requests = requests.clone();
                let responses = responses.clone();
                Arc::new(move |_, request: &JsonHttpRequest| {
                    requests
                        .lock()
                        .expect("requests lock should not be poisoned")
                        .push(request.clone());
                    Ok(responses
                        .lock()
                        .expect("responses lock should not be poisoned")
                        .pop_front()
                        .expect("unexpected transport call"))
                })
            }),
    ));

    (
        Arc::new(TransportBackedJsonHttpClient::new(transport)),
        requests,
    )
}

fn simple_request() -> ModelRequest {
    ModelRequest {
        system_prompt: "system".to_owned(),
        messages: vec![ConversationMessage::user("hello")],
        tools: Vec::new(),
        response_schema: None,
    }
}

fn google_response_body(raw_reason: Option<&str>, include_tool: bool) -> serde_json::Value {
    let parts = if include_tool {
        json!([{
            "functionCall": {
                "id": "call_1",
                "name": "memory_read",
                "args": { "path": "root" }
            }
        }])
    } else {
        json!([{ "text": "Done" }])
    };
    let mut candidate = json!({
        "content": {
            "parts": parts
        }
    });
    if let Some(raw_reason) = raw_reason {
        candidate["finishReason"] = json!(raw_reason);
    }
    json!({
        "candidates": [candidate]
    })
}
