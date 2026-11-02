//! Header forwarding, sanitized persistence, and replay through each transport path.

use std::sync::{Arc, Mutex};

use reqwest::header::HeaderValue;
use serde_json::json;
use unimock::{MockFn, Unimock, matching};

use crate::{
    DynJsonHttpSseStream, Error, JsonHttpHeaders, JsonHttpRequest, JsonHttpResponse,
    JsonHttpSseStreamMock, JsonHttpTransport, JsonHttpTransportMock,
};

use super::recording_tests::request;
use super::{MemoryRecordingLog, Recording, RecordingLog, RecordingTransport, ReplayTransport};

#[derive(Clone, Copy, Debug)]
enum Path {
    Json,
    Bytes,
    Sse,
    Rejected,
}

async fn headers(
    transport: &dyn JsonHttpTransport,
    request: &JsonHttpRequest,
    path: Path,
) -> JsonHttpHeaders {
    match path {
        Path::Json => transport.execute(request).await.unwrap().headers,
        Path::Bytes => transport.execute_bytes(request).await.unwrap().headers,
        Path::Sse => transport
            .execute_sse(request)
            .await
            .unwrap()
            .headers()
            .clone(),
        Path::Rejected => {
            let Err(Error::HttpStatus { headers, .. }) = transport.execute_sse(request).await
            else {
                panic!("expected status error")
            };
            *headers
        }
    }
}

#[tokio::test]
async fn recording_and_replay_keep_safe_headers_and_forward_original_headers() {
    let original = JsonHttpHeaders::from_iter([
        (
            "request-id".parse().unwrap(),
            HeaderValue::from_static("req-1"),
        ),
        (
            "retry-after".parse().unwrap(),
            HeaderValue::from_static("7"),
        ),
        (
            "set-cookie".parse().unwrap(),
            HeaderValue::from_static("response-secret"),
        ),
        (
            "x-echo".parse().unwrap(),
            HeaderValue::from_static("sentinel-private-key"),
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
    safe.insert("set-cookie", HeaderValue::from_static("[redacted]"));
    safe.insert("x-echo", HeaderValue::from_static("[redacted]"));
    for path in [Path::Json, Path::Bytes, Path::Sse, Path::Rejected] {
        let upstream = match path {
            Path::Json => Unimock::new(
                JsonHttpTransportMock::execute
                    .some_call(matching!(_))
                    .returns(Ok(JsonHttpResponse {
                        status: 200,
                        headers: original.clone(),
                        body: json!({}),
                    })),
            ),
            Path::Bytes => Unimock::new(
                JsonHttpTransportMock::execute_bytes
                    .some_call(matching!(_))
                    .returns(Ok(JsonHttpResponse {
                        status: 200,
                        headers: original.clone(),
                        body: vec![0, 255],
                    })),
            ),
            Path::Sse => {
                let stream = Mutex::new(Some(Unimock::new(
                    JsonHttpSseStreamMock::headers
                        .each_call(matching!())
                        .returns(original.clone()),
                )));
                Unimock::new(
                    JsonHttpTransportMock::execute_sse
                        .some_call(matching!(_))
                        .answers_arc(Arc::new(move |_, _| {
                            Ok(Box::new(stream.lock().unwrap().take().unwrap())
                                as DynJsonHttpSseStream)
                        })),
                )
            }
            Path::Rejected => {
                let original = original.clone();
                Unimock::new(
                    JsonHttpTransportMock::execute_sse
                        .some_call(matching!(_))
                        .answers_arc(Arc::new(move |_, _| {
                            Err(Error::HttpStatus {
                                status: 429,
                                headers: Box::new(original.clone()),
                                body: json!({}),
                            })
                        })),
                )
            }
        };
        let log = Arc::new(MemoryRecordingLog::default());
        let recorder = RecordingTransport::new(Arc::new(upstream), log.clone());
        assert_eq!(
            headers(&recorder, &request(), path).await,
            original,
            "{path:?}"
        );
        let encoded = serde_json::to_string(&log.snapshot()).unwrap();
        assert!(!encoded.contains("response-secret"), "{path:?}");
        assert!(!encoded.contains("sentinel-private-key"), "{path:?}");
        let decoded = serde_json::from_str::<Vec<Recording>>(&encoded).unwrap();
        let replay = ReplayTransport::new(decoded);
        assert_eq!(headers(&replay, &request(), path).await, safe, "{path:?}");
    }
}
