//! Shared DeepSeek provider test helpers.

use std::sync::{Arc, Mutex};

use ai_interface::{ConversationMessage, ModelRequest};
use json_http::{
    JsonHttpClient, JsonHttpRequest, JsonHttpResponse, JsonHttpTransportMock,
    TransportBackedJsonHttpClient,
};
use serde_json::{Value, json};
use unimock::{MockFn, Unimock, matching};

pub(super) type RecordedRequests = Arc<Mutex<Vec<JsonHttpRequest>>>;

pub(super) fn unused_http_client() -> Arc<dyn JsonHttpClient> {
    Arc::new(TransportBackedJsonHttpClient::new(Arc::new(Unimock::new(
        (),
    ))))
}

pub(super) fn recording_http_client(
    response: JsonHttpResponse<Value>,
) -> (Arc<dyn JsonHttpClient>, RecordedRequests) {
    let requests = Arc::new(Mutex::new(Vec::new()));
    let transport = Arc::new(Unimock::new(
        JsonHttpTransportMock::execute
            .each_call(matching!(_))
            .answers_arc({
                let requests = requests.clone();
                Arc::new(move |_, request: &JsonHttpRequest| {
                    requests
                        .lock()
                        .expect("requests lock should not be poisoned")
                        .push(request.clone());
                    Ok(response.clone())
                })
            }),
    ));

    (
        Arc::new(TransportBackedJsonHttpClient::new(transport)),
        requests,
    )
}

pub(super) fn transport_failure_http_client(message: &'static str) -> Arc<dyn JsonHttpClient> {
    let transport = Arc::new(Unimock::new(
        JsonHttpTransportMock::execute
            .next_call(matching!(_))
            .answers_arc(Arc::new(move |_, _| {
                Err(json_http::Error::transport(message))
            })),
    ));
    Arc::new(TransportBackedJsonHttpClient::new(transport))
}

pub(super) fn successful_response(content: Option<&str>) -> JsonHttpResponse<Value> {
    JsonHttpResponse {
        status: 200,
        body: json!({
            "choices": [{
                "finish_reason": "stop",
                "message": {
                    "content": content
                }
            }]
        }),
    }
}

pub(super) fn simple_request() -> ModelRequest {
    ModelRequest {
        system_prompt: "system".to_owned(),
        messages: vec![ConversationMessage::user("hello")],
        tools: Vec::new(),
        response_schema: None,
    }
}
