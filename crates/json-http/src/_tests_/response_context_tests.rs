//! Typed response decoding retains the response facts and original error.

use std::sync::Arc;

use reqwest::header::HeaderValue;
use serde::Deserialize;
use serde_json::json;
use unimock::{MockFn, Unimock, matching};

use crate::{
    Error, JsonHttpClient, JsonHttpHeaders, JsonHttpResponse, JsonHttpTransportMock,
    TransportBackedJsonHttpClient,
};

#[derive(Debug, Deserialize)]
struct TypedResponse {
    _ok: bool,
}

#[tokio::test]
async fn typed_decode_failure_retains_headers_and_its_typed_source() {
    let headers = JsonHttpHeaders::from_iter([(
        "request-id".parse().unwrap(),
        HeaderValue::from_static("req-decode"),
    )]);
    let client = TransportBackedJsonHttpClient::new(Arc::new(Unimock::new(
        JsonHttpTransportMock::execute
            .some_call(matching!(_))
            .returns(Ok(JsonHttpResponse {
                status: 200,
                headers: headers.clone(),
                body: json!({"_ok":"invalid"}),
            })),
    )));
    let error = client
        .get("https://example.com/typed")
        .send::<TypedResponse>()
        .await
        .unwrap_err();
    assert_eq!(error.response_headers(), Some(&headers));
    assert!(!error.is_transport_failure());
    assert!(
        matches!(error.cause(), Error::DeserializeResponse { body, source } if body == &json!({"_ok":"invalid"}) && source.is_data())
    );
    assert!(
        error
            .to_string()
            .contains("failed to deserialize response body")
    );
    assert!(matches!(
        error.into_cause(),
        Error::DeserializeResponse { .. }
    ));
}
