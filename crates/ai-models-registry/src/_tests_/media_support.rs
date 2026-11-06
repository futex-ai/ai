//! In-memory JSON and byte transports for modality factory requests.

use std::sync::{Arc, Mutex};

use json_http::{
    DynJsonHttpClient, JsonHttpRequest, JsonHttpResponse, JsonHttpTransportMock,
    TransportBackedJsonHttpClient,
};
use serde_json::Value;
use unimock::{MockFn, Unimock, matching};

pub(super) type Requests = Arc<Mutex<Vec<JsonHttpRequest>>>;

#[cfg(any(feature = "google", feature = "openai"))]
pub(super) fn json_client(body: Value) -> (DynJsonHttpClient, Requests) {
    let requests = Arc::new(Mutex::new(Vec::new()));
    let transport = Unimock::new(
        JsonHttpTransportMock::execute
            .some_call(matching!(_))
            .answers_arc({
                let requests = requests.clone();
                Arc::new(move |_, request: &JsonHttpRequest| {
                    requests.lock().unwrap().push(request.clone());
                    Ok(JsonHttpResponse {
                        status: 200,
                        headers: Default::default(),
                        body: body.clone(),
                    })
                })
            }),
    );
    (
        Arc::new(TransportBackedJsonHttpClient::new(Arc::new(transport))),
        requests,
    )
}

#[cfg(any(feature = "openai", feature = "typesafe"))]
pub(super) fn bytes_client(body: Value) -> (DynJsonHttpClient, Requests) {
    let requests = Arc::new(Mutex::new(Vec::new()));
    let transport = Unimock::new(
        JsonHttpTransportMock::execute_bytes
            .some_call(matching!(_))
            .answers_arc({
                let requests = requests.clone();
                Arc::new(move |_, request: &JsonHttpRequest| {
                    requests.lock().unwrap().push(request.clone());
                    Ok(JsonHttpResponse {
                        status: 200,
                        headers: Default::default(),
                        body: serde_json::to_vec(&body).unwrap(),
                    })
                })
            }),
    );
    (
        Arc::new(TransportBackedJsonHttpClient::new(Arc::new(transport))),
        requests,
    )
}
