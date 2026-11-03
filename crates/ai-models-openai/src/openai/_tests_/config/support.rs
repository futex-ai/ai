//! In-memory HTTP, auth, and polling fixtures for provider configuration.

use std::{
    collections::{BTreeMap, VecDeque},
    sync::{Arc, Mutex},
};

use ai_models_core::ProviderConfig;
use json_http::{
    DynJsonHttpAuth, DynJsonHttpClient, DynJsonHttpTransport, JsonHttpAuthMock, JsonHttpBody,
    JsonHttpMultipartField, JsonHttpRequest, JsonHttpResponse, JsonHttpTransportMock,
    TransportBackedJsonHttpClient,
};
use serde_json::Value;
use unimock::{MockFn, Unimock, matching};

use crate::PROVIDER_ID;

pub(super) type RecordedRequests = Arc<Mutex<Vec<JsonHttpRequest>>>;

pub(super) fn config(base: Option<&str>) -> ProviderConfig {
    let config = ProviderConfig::new(PROVIDER_ID, "config-key")
        .with_header("X-Tenant", "tenant-value")
        .with_header("aUtHoRiZaTiOn", "extra-wrong");
    match base {
        Some(base) => config.with_base_url(base),
        None => config,
    }
}

pub(super) fn rotating_auth() -> DynJsonHttpAuth {
    let calls = Arc::new(Mutex::new(0));
    Arc::new(Unimock::new(
        JsonHttpAuthMock::apply_headers
            .each_call(matching!(_))
            .answers_arc(Arc::new(
                move |_, headers: &mut BTreeMap<String, String>| {
                    let mut calls = calls.lock().unwrap();
                    *calls += 1;
                    headers.insert(
                        "Authorization".to_owned(),
                        format!("Bearer fresh-{}", *calls),
                    );
                    Ok(())
                },
            )),
    ))
}

pub(super) fn assert_headers(request: &JsonHttpRequest, auth: &str) {
    let authorization: Vec<_> = request
        .headers
        .iter()
        .filter(|(name, _)| name.eq_ignore_ascii_case("authorization"))
        .map(|(_, value)| value.as_str())
        .collect();
    assert_eq!(authorization, [auth]);
    assert_eq!(
        request.headers.get("X-Tenant").map(String::as_str),
        Some("tenant-value")
    );
}

pub(super) fn json_response(body: Value) -> JsonHttpResponse<Value> {
    JsonHttpResponse {
        status: 200,
        headers: Default::default(),
        body,
    }
}

pub(super) fn buffered_client(
    json: Vec<JsonHttpResponse<Value>>,
    bytes: Vec<JsonHttpResponse<Vec<u8>>>,
) -> (DynJsonHttpClient, RecordedRequests) {
    let has_json = !json.is_empty();
    let has_bytes = !bytes.is_empty();
    let requests = Arc::new(Mutex::new(Vec::new()));
    let json = Arc::new(Mutex::new(VecDeque::from(json)));
    let bytes = Arc::new(Mutex::new(VecDeque::from(bytes)));
    let json = JsonHttpTransportMock::execute
        .each_call(matching!(_))
        .answers_arc({
            let requests = requests.clone();
            Arc::new(move |_, request: &JsonHttpRequest| {
                requests.lock().unwrap().push(request.clone());
                Ok(json.lock().unwrap().pop_front().expect("JSON response"))
            })
        });
    let bytes = JsonHttpTransportMock::execute_bytes
        .each_call(matching!(_))
        .answers_arc({
            let requests = requests.clone();
            Arc::new(move |_, request: &JsonHttpRequest| {
                requests.lock().unwrap().push(request.clone());
                Ok(bytes.lock().unwrap().pop_front().expect("byte response"))
            })
        });
    let transport: DynJsonHttpTransport = match (has_json, has_bytes) {
        (true, true) => Arc::new(Unimock::new((json, bytes))),
        (true, false) => Arc::new(Unimock::new(json)),
        (false, true) => Arc::new(Unimock::new(bytes)),
        (false, false) => Arc::new(Unimock::new(())),
    };
    (
        Arc::new(TransportBackedJsonHttpClient::new(transport)),
        requests,
    )
}

pub(super) fn multipart_fields(request: &JsonHttpRequest) -> &[JsonHttpMultipartField] {
    let Some(JsonHttpBody::Multipart(body)) = &request.body else {
        panic!("request must use multipart");
    };
    &body.fields
}
