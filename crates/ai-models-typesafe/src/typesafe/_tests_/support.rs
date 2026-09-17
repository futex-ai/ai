//! Shared TypeSafe provider test helpers.

use std::collections::{BTreeMap, VecDeque};
use std::sync::{Arc, Mutex};

use ai_interface::{JudgmentQuestion, JudgmentRequest};
use json_http::{
    DynJsonHttpClient, JsonHttpRequest, JsonHttpResponse, JsonHttpTransportMock,
    TransportBackedJsonHttpClient,
};
use serde_json::{Value, json};
use unimock::{MockFn, Unimock, matching};

/// Requests captured by a mocked transport.
pub(super) type RecordedRequests = Arc<Mutex<Vec<JsonHttpRequest>>>;

/// Returns a client whose transport must not be called.
pub(super) fn unused_http_client() -> DynJsonHttpClient {
    Arc::new(TransportBackedJsonHttpClient::new(Arc::new(Unimock::new(
        (),
    ))))
}

/// Returns a client that records one request and yields one response.
pub(super) fn recording_http_client(
    response: JsonHttpResponse<Value>,
) -> (DynJsonHttpClient, RecordedRequests) {
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
                        .expect("request lock should be valid")
                        .push(request.clone());
                    Ok(responses
                        .lock()
                        .expect("response lock should be valid")
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

/// Returns a client that fails its only transport call.
pub(super) fn transport_failure_http_client(message: &'static str) -> DynJsonHttpClient {
    let transport = Arc::new(Unimock::new(
        JsonHttpTransportMock::execute
            .next_call(matching!(_))
            .returns(Err(json_http::Error::transport(message))),
    ));
    Arc::new(TransportBackedJsonHttpClient::new(transport))
}

/// Builds the smallest valid condition request fixture.
pub(super) fn simple_request() -> JudgmentRequest {
    JudgmentRequest {
        state: "A payout has been delayed for three days.".into(),
        questions: BTreeMap::from([(
            "is_urgent".to_owned(),
            JudgmentQuestion::condition("Is this urgent?", None),
        )]),
    }
}

/// Builds a successful response for the simple request fixture.
pub(super) fn successful_response() -> JsonHttpResponse<Value> {
    JsonHttpResponse {
        status: 200,
        body: json!({
            "model": "jev-1.13.0",
            "answers": {
                "is_urgent": {"type": "noul", "noul": 0.92}
            },
            "usage": {"input_tokens": 12, "output_tokens": 3}
        }),
    }
}
