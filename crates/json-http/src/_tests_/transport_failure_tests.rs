//! Stable transport classification used by deferred retrieval and adapters.

use std::time::Duration;

use reqwest::IntoUrl;
use serde_json::json;

use crate::Error;

fn request_error(url: impl IntoUrl) -> reqwest::Error {
    url.into_url().unwrap_err()
}

#[test]
fn shared_transport_classification_keeps_network_and_local_failures_separate() {
    let cases = [
        (Error::transport("network failed"), true),
        (
            Error::ReqwestTransport {
                source: request_error("://invalid"),
            },
            true,
        ),
        (
            Error::SseTransport {
                events_received: 2,
                source: request_error("://invalid"),
            },
            true,
        ),
        (Error::auth("denied"), false),
        (
            Error::HttpStatus {
                status: 503,
                headers: Default::default(),
                body: json!({}),
            },
            false,
        ),
        (Error::SseUnsupported, false),
        (
            Error::DeadlineExceeded {
                timeout: Duration::from_secs(10),
                events_received: 0,
            },
            false,
        ),
        (
            Error::IdleTimeout {
                idle: Duration::from_secs(1),
                events_received: 0,
            },
            false,
        ),
        (
            Error::SerializeRequest {
                source: serde_json::from_str::<bool>("invalid").unwrap_err(),
            },
            false,
        ),
    ];
    for (error, expected) in cases {
        assert_eq!(error.is_transport_failure(), expected, "{error:?}");
    }
}
