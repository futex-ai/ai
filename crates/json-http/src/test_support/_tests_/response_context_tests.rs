//! Response-context failures retain safe headers and their replay failure class.

use std::sync::Arc;

use reqwest::header::HeaderValue;
use unimock::{MockFn, Unimock, matching};

use crate::{Error, JsonHttpHeaders, JsonHttpTransport, JsonHttpTransportMock};

use super::recording_tests::request;
use super::{MemoryRecordingLog, Recording, RecordingLog, RecordingTransport, ReplayTransport};

#[tokio::test]
async fn response_context_failures_record_safe_headers_and_replay_the_cause() {
    let original = JsonHttpHeaders::from_iter([
        (
            "request-id".parse().unwrap(),
            HeaderValue::from_static("req-context"),
        ),
        (
            "x-echo".parse().unwrap(),
            HeaderValue::from_static("sentinel-private-key"),
        ),
        (
            "set-cookie".parse().unwrap(),
            HeaderValue::from_static("response-secret"),
        ),
        (
            "x-binary".parse().unwrap(),
            HeaderValue::from_bytes(&[255]).unwrap(),
        ),
        (
            "x-repeated".parse().unwrap(),
            HeaderValue::from_static("first"),
        ),
        (
            "x-repeated".parse().unwrap(),
            HeaderValue::from_static("second"),
        ),
    ]);
    let mut safe = original.clone();
    safe.as_raw_mut()
        .insert("x-echo", HeaderValue::from_static("[redacted]"));
    safe.as_raw_mut()
        .insert("set-cookie", HeaderValue::from_static("[redacted]"));
    let upstream_headers = original.clone();
    let upstream = Unimock::new(
        JsonHttpTransportMock::execute_sse
            .some_call(matching!(_))
            .answers_arc(Arc::new(move |_, _| {
                Err(Error::InvalidSseContentType {
                    content_type: Some("application/json".to_owned()),
                }
                .with_response_headers(upstream_headers.clone()))
            })),
    );
    let log = Arc::new(MemoryRecordingLog::default());
    let recorder = RecordingTransport::new(Arc::new(upstream), log.clone());
    let error = recorder.execute_sse(&request()).await.err().unwrap();
    assert_eq!(error.response_headers(), Some(&original));
    assert!(
        matches!(error.cause(), Error::InvalidSseContentType { content_type } if content_type.as_deref() == Some("application/json"))
    );
    let encoded = serde_json::to_vec(&log.snapshot()).unwrap();
    let decoded: Vec<Recording> = serde_json::from_slice(&encoded).unwrap();
    let replay = ReplayTransport::new(decoded);
    let error = replay.execute_sse(&request()).await.err().unwrap();
    assert_eq!(error.response_headers(), Some(&safe));
    assert!(matches!(error.cause(), Error::InvalidSseContentType { .. }));
    assert!(!error.is_transport_failure());
}
