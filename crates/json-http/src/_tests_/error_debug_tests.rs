//! HTTP error diagnostics omit provider-controlled values.

use std::sync::Arc;

use reqwest::IntoUrl;
use serde_json::json;
use unimock::{MockFn, Unimock, matching};

use crate::{
    Error, JsonHttpClient, JsonHttpHeaders, JsonHttpTransportMock, TransportBackedJsonHttpClient,
};

#[test]
fn http_error_debug_reports_structure_without_header_values_or_body_text() {
    let mut headers = JsonHttpHeaders::new();
    headers
        .as_raw_mut()
        .insert("x-request-id", "placeholder-header-key".parse().unwrap());
    let error = Error::HttpStatus {
        status: 401,
        headers: Box::new(headers.clone()),
        body: json!({"message": "placeholder-body-key"}),
    };
    for error in [
        error,
        Error::DeserializeResponse {
            body: json!({"message": "placeholder-body-key"}),
            source: serde_json::from_value::<bool>(json!("placeholder-body-key")).unwrap_err(),
        }
        .with_response_headers(headers),
    ] {
        let debug = format!("{error:?}");
        assert!(!debug.contains("placeholder-header-key"), "{debug}");
        assert!(!debug.contains("placeholder-body-key"), "{debug}");
        assert!(debug.contains("x-request-id"), "{debug}");
        assert!(debug.contains("body_size"), "{debug}");
    }
}

#[tokio::test]
async fn stored_transport_urls_omit_user_info_queries_and_fragments() {
    let source = invalid_url("://invalid").with_url(
        "https://placeholder-user:placeholder-password@provider.test/file?sig=placeholder-token#placeholder-fragment"
            .parse().unwrap(),
    );
    let http = TransportBackedJsonHttpClient::new(Arc::new(Unimock::new(
        JsonHttpTransportMock::execute_bytes
            .some_call(matching!(_))
            .returns(Err(Error::ReqwestTransport { source })),
    )));
    let error = http
        .get("https://provider.test/file")
        .send_bytes()
        .await
        .unwrap_err();
    let Error::ReqwestTransport { source } = error else {
        panic!("typed transport cause lost")
    };
    assert_eq!(source.url().unwrap().as_str(), "https://provider.test/file");
}

fn invalid_url(url: impl IntoUrl) -> reqwest::Error {
    url.into_url().unwrap_err()
}
