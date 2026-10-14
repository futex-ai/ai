//! Shared OpenAI Decisions test helpers.

use std::collections::{BTreeMap, VecDeque};
use std::sync::{Arc, Mutex};

use ai_interface::{JudgmentContent, JudgmentQuestion, JudgmentRequest};
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
    response: JsonHttpResponse<Vec<u8>>,
) -> (DynJsonHttpClient, RecordedRequests) {
    let requests = Arc::new(Mutex::new(Vec::new()));
    let responses = Arc::new(Mutex::new(VecDeque::from([response])));
    let transport = Arc::new(Unimock::new(
        JsonHttpTransportMock::execute_bytes
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
        JsonHttpTransportMock::execute_bytes
            .next_call(matching!(_))
            .returns(Err(json_http::Error::transport(message))),
    ));
    Arc::new(TransportBackedJsonHttpClient::new(transport))
}

/// Builds a byte response from one JSON fixture.
pub(super) fn json_response(status: u16, body: Value) -> JsonHttpResponse<Vec<u8>> {
    JsonHttpResponse {
        status,
        body: serde_json::to_vec(&body).expect("JSON fixture should serialize"),
    }
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

/// Builds a request with one question of each kind.
pub(super) fn full_request() -> JudgmentRequest {
    JudgmentRequest {
        state: "ticket".into(),
        questions: BTreeMap::from([
            (
                "choice".to_owned(),
                JudgmentQuestion::choice(
                    "Team?",
                    [
                        ("billing", Some(JudgmentContent::from("Payments"))),
                        ("technical", None),
                    ],
                ),
            ),
            (
                "condition".to_owned(),
                JudgmentQuestion::condition("Urgent?", None),
            ),
            (
                "score".to_owned(),
                JudgmentQuestion::score("Frustration?", ["calm", "frustrated", "angry"]),
            ),
        ]),
    }
}

/// Builds a valid provider body for [`full_request`].
pub(super) fn full_body() -> Value {
    json!({
        "model": "gpt-6-luna",
        "answers": [
            {"type": "predicate", "name": "condition", "probability": 0.92, "ignored": true},
            {
                "type": "choice",
                "name": "choice",
                "choice": "billing",
                "probabilities": [
                    {"value": "billing", "probability": 0.84},
                    {"value": "technical", "probability": 0.16}
                ],
                "confidence": 0.6,
                "ignored": "field"
            },
            {
                "type": "score",
                "name": "score",
                "score": 1.6,
                "probabilities": [
                    {"value": 0, "label": "calm", "probability": 0.05},
                    {"value": 1, "label": "frustrated", "probability": 0.3},
                    {"value": 2, "label": "angry", "probability": 0.65}
                ],
                "confidence": 0.78
            }
        ],
        "usage": {
            "input_tokens": 312,
            "input_tokens_details": {"cached_tokens": 0, "cache_write_tokens": 0},
            "output_tokens": 0,
            "output_tokens_details": {"reasoning_tokens": 0},
            "total_tokens": 312
        }
    })
}

/// Builds a successful response for the simple request fixture.
pub(super) fn successful_response() -> JsonHttpResponse<Vec<u8>> {
    json_response(
        200,
        json!({
            "model": "gpt-6-luna",
            "answers": [
                {"type": "predicate", "name": "is_urgent", "probability": 0.92}
            ],
            "usage": {"input_tokens": 12, "output_tokens": 0, "total_tokens": 12}
        }),
    )
}
