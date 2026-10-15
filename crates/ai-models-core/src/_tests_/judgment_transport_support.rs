//! Shared helpers for judgment transport tests.

use std::collections::{BTreeMap, VecDeque};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use ai_interface::{JudgmentError, JudgmentResult};
use json_http::{
    DynJsonHttpAuth, DynJsonHttpClient, JsonHttpRequest, JsonHttpResponse, JsonHttpTransportMock,
    StaticHeaderAuth, TransportBackedJsonHttpClient,
};
use serde_json::Value;
use unimock::{MockFn, Unimock, matching};

use super::{HttpJudgmentTransport, JudgmentEndpoint, JudgmentHttpResponse};

/// Requests captured by a mocked transport.
pub(super) type RecordedRequests = Arc<Mutex<Vec<JsonHttpRequest>>>;

/// Provider name reported by the test endpoint.
pub(super) const PROVIDER: &str = "test-provider";

/// Model id used by every test transport.
pub(super) const MODEL_ID: &str = "judge-model";

/// Bearer token sent by [`bearer_transport`].
pub(super) const TOKEN: &str = "core-secret";

/// Test endpoint whose extractor reads `{"failure": {"text": ...}}`.
pub(super) const TEST_ENDPOINT: JudgmentEndpoint = JudgmentEndpoint {
    provider: PROVIDER,
    url: "https://judge.test/v1/judge",
    timeout: Duration::from_secs(45),
    failure_message: test_failure_message,
};

/// Extracts `failure.text`, keeps string bodies, and renders other JSON.
fn test_failure_message(body: &Value) -> String {
    if let Some(text) = body.pointer("/failure/text").and_then(Value::as_str) {
        return text.to_owned();
    }
    match body {
        Value::String(message) => message.clone(),
        body => body.to_string(),
    }
}

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

/// Returns a client whose only transport call fails with `error`.
pub(super) fn failing_http_client(error: json_http::Error) -> DynJsonHttpClient {
    let transport = Arc::new(Unimock::new(
        JsonHttpTransportMock::execute_bytes
            .next_call(matching!(_))
            .returns(Err(error)),
    ));
    Arc::new(TransportBackedJsonHttpClient::new(transport))
}

/// Builds a raw byte response.
pub(super) fn bytes_response(status: u16, body: &[u8]) -> JsonHttpResponse<Vec<u8>> {
    JsonHttpResponse {
        status,
        body: body.to_vec(),
    }
}

/// Builds a transport that authenticates with `Bearer core-secret`.
pub(super) fn bearer_transport(http_client: DynJsonHttpClient) -> HttpJudgmentTransport {
    let auth: DynJsonHttpAuth = Arc::new(StaticHeaderAuth::new(BTreeMap::from([(
        "Authorization".to_owned(),
        ["Bearer ", TOKEN].concat(),
    )])));
    HttpJudgmentTransport::new(TEST_ENDPOINT, MODEL_ID, http_client, auth)
}

/// Returns the error of a call that must fail.
pub(super) fn expect_error(result: JudgmentResult<JudgmentHttpResponse>) -> JudgmentError {
    match result {
        Ok(response) => panic!(
            "expected an error, got a {}-byte success body",
            response.body.len()
        ),
        Err(error) => error,
    }
}

/// Returns the provider, model id, and message of a provider-class error.
pub(super) fn provider_fields(error: &JudgmentError) -> (&str, &str, &str) {
    match error {
        JudgmentError::RateLimited {
            provider,
            model_id,
            message,
        }
        | JudgmentError::TransientProvider {
            provider,
            model_id,
            message,
        }
        | JudgmentError::Provider {
            provider,
            model_id,
            message,
        } => (provider, model_id, message),
        error => panic!("expected a provider-class error, got {error}"),
    }
}
