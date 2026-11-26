//! In-memory transport and request fixtures for Decisions tests.

use std::{
    collections::BTreeMap,
    sync::{Arc, Mutex},
};

use ai_interface::{JudgmentQuestion, JudgmentRequest, ModelSpec};
use ai_models_core::ProviderConfig;
use json_http::{
    DynJsonHttpClient, JsonHttpHeaders, JsonHttpRequest, JsonHttpResponse, JsonHttpTransportMock,
    TransportBackedJsonHttpClient,
};
use serde_json::{Value, json};
use unimock::{MockFn, Unimock, matching};

use crate::{CUSTOM_DEFAULTS, PROVIDER_ID};

use super::super::OpenAiJudgmentModel;

pub(super) const MODEL_ID: &str = "fixture-decisions";
pub(super) const API_KEY: &str = "fixture-api-key";

pub(super) type Requests = Arc<Mutex<Vec<JsonHttpRequest>>>;

pub(super) fn spec() -> ModelSpec {
    ModelSpec::custom(PROVIDER_ID, MODEL_ID, &CUSTOM_DEFAULTS)
}

pub(super) fn config() -> ProviderConfig {
    ProviderConfig::new(PROVIDER_ID, API_KEY)
}

pub(super) fn client(
    response: json_http::Result<JsonHttpResponse<Vec<u8>>>,
) -> (DynJsonHttpClient, Requests) {
    let requests = Arc::new(Mutex::new(Vec::new()));
    let captured = requests.clone();
    let reply = Mutex::new(Some(response));
    let transport = Unimock::new(
        JsonHttpTransportMock::execute_bytes
            .some_call(matching!(_))
            .answers_arc(Arc::new(move |_, request: &JsonHttpRequest| {
                captured.lock().unwrap().push(request.clone());
                reply.lock().unwrap().take().expect("one transport call")
            })),
    );
    (
        Arc::new(TransportBackedJsonHttpClient::new(Arc::new(transport))),
        requests,
    )
}

pub(super) fn model(body: Value) -> (OpenAiJudgmentModel, Requests) {
    let (http, requests) = client(Ok(response(200, body)));
    (
        OpenAiJudgmentModel::from_config(http, spec(), config()),
        requests,
    )
}

pub(super) fn response(status: u16, body: Value) -> JsonHttpResponse<Vec<u8>> {
    JsonHttpResponse {
        status,
        headers: JsonHttpHeaders::default(),
        body: serde_json::to_vec(&body).unwrap(),
    }
}

pub(super) fn request(question: JudgmentQuestion) -> JudgmentRequest {
    JudgmentRequest {
        input: "The ticket reports a payout failure.".into(),
        questions: BTreeMap::from([("caller-id".to_owned(), question)]),
    }
}

pub(super) fn predicate() -> JudgmentRequest {
    request(JudgmentQuestion::condition("Is this urgent?", None))
}

pub(super) fn predicate_response() -> Value {
    json!({"model":"decisions-release","answers":[{"type":"predicate","name":"q0","probability":0.9}]})
}

pub(super) fn sent_request(requests: &Requests) -> JsonHttpRequest {
    let requests = requests.lock().unwrap();
    assert_eq!(requests.len(), 1);
    requests[0].clone()
}

pub(super) fn request_body(request: &JsonHttpRequest) -> Value {
    serde_json::from_slice(request.body.as_ref().unwrap().as_json_bytes().unwrap()).unwrap()
}
