//! Tests for request builder serialization and auth header wiring.

use std::{
    collections::{BTreeMap, VecDeque},
    sync::{Arc, Mutex},
    time::Duration,
};

use serde::{Deserialize, Serialize};
use serde_json::json;
use unimock::{MockFn, Unimock, matching};

use crate::{
    JsonHttpBody, JsonHttpClient, JsonHttpMethod, JsonHttpMultipart, JsonHttpMultipartField,
    JsonHttpRequest, JsonHttpResponse, JsonHttpTransportMock, StaticHeaderAuth,
    TransportBackedJsonHttpClient,
};

type RecordedRequests = Arc<Mutex<Vec<JsonHttpRequest>>>;

#[derive(Debug, Deserialize, PartialEq)]
struct TypedResponse {
    ok: bool,
}

#[derive(Debug, Serialize)]
struct TypedRequest {
    prompt: String,
}

#[tokio::test]
async fn builder_serializes_typed_body_and_applies_auth_headers() {
    let (client, requests) = recording_client(JsonHttpResponse {
        status: 200,
        body: json!({ "ok": true }),
    });
    let auth = Arc::new(StaticHeaderAuth::new(BTreeMap::from([(
        "Authorization".to_owned(),
        "Bearer token".to_owned(),
    )])));

    let response = client
        .post("https://example.com/v1/chat")
        .header("x-trace-id", "trace-1")
        .auth(auth)
        .json(TypedRequest {
            prompt: "hello".to_owned(),
        })
        .expect("request body should serialize")
        .send::<TypedResponse>()
        .await
        .expect("request should succeed");

    assert_eq!(response.status, 200);
    assert_eq!(response.body, TypedResponse { ok: true });

    let requests = requests
        .lock()
        .expect("requests lock should not be poisoned");
    assert_eq!(requests.len(), 1);
    assert_eq!(requests[0].method, JsonHttpMethod::Post);
    assert_eq!(requests[0].url, "https://example.com/v1/chat");
    assert_eq!(
        requests[0].headers.get("Authorization"),
        Some(&"Bearer token".to_owned())
    );
    assert_eq!(
        requests[0].headers.get("x-trace-id"),
        Some(&"trace-1".to_owned())
    );
    assert_eq!(
        requests[0].body,
        Some(JsonHttpBody::Json(json!({
            "prompt": "hello"
        })))
    );
}

#[tokio::test]
async fn builder_attaches_multipart_body_and_applies_auth_headers() {
    let (client, requests) = recording_client(JsonHttpResponse {
        status: 200,
        body: json!({ "ok": true }),
    });
    let auth = Arc::new(StaticHeaderAuth::bearer_token("token"));

    let response = client
        .post("https://example.com/v1/audio")
        .auth(auth)
        .multipart(vec![
            JsonHttpMultipartField::bytes("audio", b"audio-bytes".to_vec())
                .filename("voice.wav")
                .content_type("audio/wav"),
        ])
        .send::<TypedResponse>()
        .await
        .expect("request should succeed");

    assert_eq!(response.body, TypedResponse { ok: true });

    let requests = requests
        .lock()
        .expect("requests lock should not be poisoned");
    assert_eq!(requests.len(), 1);
    assert_eq!(
        requests[0].headers.get("Authorization"),
        Some(&"Bearer token".to_owned())
    );
    assert_eq!(
        requests[0].body,
        Some(JsonHttpBody::Multipart(JsonHttpMultipart {
            fields: vec![
                JsonHttpMultipartField::bytes("audio", b"audio-bytes".to_vec())
                    .filename("voice.wav")
                    .content_type("audio/wav"),
            ],
        }))
    );
}

#[tokio::test]
async fn builder_can_return_raw_json_value_response() {
    let (client, _) = recording_client(JsonHttpResponse {
        status: 202,
        body: json!({ "queued": true }),
    });

    let response = client
        .get("https://example.com/v1/jobs/1")
        .send_value()
        .await
        .expect("request should succeed");

    assert_eq!(response.status, 202);
    assert_eq!(response.body, json!({ "queued": true }));
}

#[tokio::test]
async fn builder_attaches_custom_timeout_to_request() {
    let (client, requests) = recording_client(JsonHttpResponse {
        status: 200,
        body: json!({ "ok": true }),
    });

    client
        .post("https://example.com/v1/slow")
        .timeout(Duration::from_secs(20 * 60))
        .send_value()
        .await
        .expect("request should succeed");

    let requests = requests
        .lock()
        .expect("requests lock should not be poisoned");
    assert_eq!(requests[0].timeout, Duration::from_secs(20 * 60));
}

fn recording_client(
    response: JsonHttpResponse<serde_json::Value>,
) -> (TransportBackedJsonHttpClient, RecordedRequests) {
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

    (TransportBackedJsonHttpClient::new(transport), requests)
}
