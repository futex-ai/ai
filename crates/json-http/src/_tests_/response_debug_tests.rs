//! Response diagnostics retain structure without exposing credentials or bodies.

use std::sync::{Arc, Mutex};

use reqwest::header::HeaderValue;
use serde_json::json;
use unimock::{MockFn, Unimock, matching};

use crate::{
    DynJsonHttpSseStream, Error, JsonHttpClient, JsonHttpHeaders, JsonHttpResponse,
    JsonHttpSseStreamMock, JsonHttpTransportMock, TransportBackedJsonHttpClient,
};

fn credential_headers() -> JsonHttpHeaders {
    JsonHttpHeaders::from_iter([
        (
            "set-cookie".parse().unwrap(),
            HeaderValue::from_static("session=placeholder-cookie-token"),
        ),
        (
            "x-echo".parse().unwrap(),
            HeaderValue::from_static("placeholder-echoed-key"),
        ),
    ])
}

fn assert_private_headers(debug: &str) {
    assert!(!debug.contains("placeholder-cookie-token"), "{debug}");
    assert!(!debug.contains("placeholder-echoed-key"), "{debug}");
    assert!(debug.contains("set-cookie"), "{debug}");
    assert!(debug.contains("x-echo"), "{debug}");
}

#[test]
fn buffered_response_debug_omits_header_values() {
    let response = JsonHttpResponse {
        status: 200,
        headers: credential_headers(),
        body: json!({"ok":true}),
    };
    assert_private_headers(&format!("{response:?}"));
    let response = JsonHttpResponse {
        status: 200,
        headers: credential_headers(),
        body: b"video".to_vec(),
    };
    assert_private_headers(&format!("{response:?}"));
}

#[tokio::test]
async fn stream_header_debug_omits_values() {
    let stream = Mutex::new(Some(Unimock::new(
        JsonHttpSseStreamMock::headers
            .some_call(matching!())
            .returns(credential_headers()),
    )));
    let client = TransportBackedJsonHttpClient::new(Arc::new(Unimock::new(
        JsonHttpTransportMock::execute_sse
            .some_call(matching!(_))
            .answers_arc(Arc::new(move |_, _| {
                Ok(Box::new(stream.lock().unwrap().take().unwrap()) as DynJsonHttpSseStream)
            })),
    )));
    let stream = client
        .get("https://provider.test/events")
        .send_sse()
        .await
        .unwrap();
    assert_private_headers(&format!("{:?}", stream.headers()));
}

#[test]
fn error_header_debug_omits_values() {
    for error in [
        Error::HttpStatus {
            status: 401,
            headers: Box::new(credential_headers()),
            body: json!({}),
        },
        Error::transport("response body read failed").with_response_headers(credential_headers()),
    ] {
        assert_private_headers(&format!("{:?}", error.response_headers().unwrap()));
    }
}

#[test]
fn buffered_unauthorized_response_debug_omits_body() {
    let response = JsonHttpResponse {
        status: 401,
        headers: JsonHttpHeaders::new(),
        body: json!({"error":"rejected placeholder-echoed-key"}),
    };
    let debug = format!("{response:?}");
    assert!(!debug.contains("placeholder-echoed-key"), "{debug}");
    assert!(debug.contains("401"), "{debug}");
    assert!(debug.contains("[redacted]"), "{debug}");
}
