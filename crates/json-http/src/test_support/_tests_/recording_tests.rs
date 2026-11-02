//! Recording boundary tests with mocked upstream transports.

use std::{collections::BTreeMap, sync::Arc, time::Duration};

use base64::{Engine, prelude::BASE64_STANDARD};
use serde_json::json;
use unimock::{MockFn, Unimock, matching};

use crate::{
    Error, JsonHttpBody, JsonHttpHeaders, JsonHttpMethod, JsonHttpMultipart,
    JsonHttpMultipartField, JsonHttpRequest, JsonHttpResponse, JsonHttpSseEvent,
    JsonHttpSseStreamMock, JsonHttpTransport, JsonHttpTransportMock,
};

use super::{
    MemoryRecordingLog, RecordedBody, RecordedFailure, RecordedResponse, Recording, RecordingLog,
    RecordingTransport, StreamEnd,
};

/// A credentialed request whose original form must still reach the upstream.
pub(super) fn request() -> JsonHttpRequest {
    JsonHttpRequest {
        method: JsonHttpMethod::Post,
        url: "https://example.com/chat".to_owned(),
        headers: BTreeMap::from([(
            "Authorization".to_owned(),
            "Bearer sentinel-private-key".to_owned(),
        )]),
        body: Some(JsonHttpBody::Json(
            json!({"prompt": "hello"}).to_string().into_bytes(),
        )),
        timeout: Duration::from_secs(60),
        idle_timeout: Some(Duration::from_secs(10)),
    }
}

#[tokio::test]
async fn forwards_original_json_and_records_no_credentials() {
    let mut request = request();
    request.url = "https://example.com/chat?key=sentinel-query-key".to_owned();
    request.headers.extend([
        ("X-API-KEY".to_owned(), "sentinel-anthropic-key".to_owned()),
        (
            "x-goog-api-key".to_owned(),
            "sentinel-google-key".to_owned(),
        ),
        ("api-key".to_owned(), "sentinel-api-key".to_owned()),
    ]);
    request.body = Some(JsonHttpBody::Json(
        json!({"prompt": "sentinel-private-key", "api_key": "body-only-secret"})
            .to_string()
            .into_bytes(),
    ));
    let original = request.clone();
    let upstream = Unimock::new(JsonHttpTransportMock::execute.some_call(matching!(_)).answers_arc(Arc::new(move |_, request| {
        assert_eq!(request, &original);
        Ok(JsonHttpResponse { headers: Default::default(), status: 201, body: json!({"echo": "sentinel-private-key", "body_echo": "body-only-secret", "authorization": "response-secret"}) })
    })));
    let log: Arc<dyn RecordingLog + Send + Sync> = Arc::new(MemoryRecordingLog::default());
    let transport = RecordingTransport::new(Arc::new(upstream), log.clone());
    let response = transport
        .execute(&request)
        .await
        .expect("forwarded response");
    assert_eq!(response.body["echo"], "sentinel-private-key");
    let recordings = log.snapshot();
    assert_eq!(
        recordings[0].request.headers.values().collect::<Vec<_>>(),
        vec!["[redacted]"; 4]
    );
    assert_eq!(
        recordings[0].response,
        RecordedResponse::Json {
            headers: Default::default(),
            status: 201,
            body: json!({"echo": "[redacted]", "body_echo": "[redacted]", "authorization": "[redacted]"})
        }
    );
    let serialized = serde_json::to_string(&recordings).expect("serializable log");
    for secret in [
        "sentinel-private-key",
        "sentinel-query-key",
        "sentinel-anthropic-key",
        "sentinel-google-key",
        "sentinel-api-key",
        "body-only-secret",
        "response-secret",
    ] {
        assert!(!serialized.contains(secret), "recording leaked {secret}");
    }
    let decoded = serde_json::from_str::<Vec<Recording>>(&serialized).expect("portable log");
    assert_eq!(decoded, recordings);
}

#[tokio::test]
async fn stores_multipart_metadata_and_base64_without_upload_bytes() {
    let mut request = request();
    request.body = Some(JsonHttpBody::Multipart(JsonHttpMultipart {
        fields: vec![
            JsonHttpMultipartField::bytes("file", b"private-upload".to_vec())
                .filename("image.png")
                .content_type("image/png"),
        ],
    }));
    let original_bytes = [vec![0, 255, 42], b"sentinel-private-key".to_vec()].concat();
    let upstream = Unimock::new(
        JsonHttpTransportMock::execute_bytes
            .some_call(matching!(_))
            .returns(Ok(JsonHttpResponse {
                headers: Default::default(),
                status: 200,
                body: original_bytes.clone(),
            })),
    );
    let log: Arc<dyn RecordingLog + Send + Sync> = Arc::new(MemoryRecordingLog::default());
    let transport = RecordingTransport::new(Arc::new(upstream), log.clone());
    let response = transport
        .execute_bytes(&request)
        .await
        .expect("bytes pass through");
    assert_eq!(response.body, original_bytes);
    let recordings = log.snapshot();
    let Some(RecordedBody::Multipart(fields)) = &recordings[0].request.body else {
        panic!("multipart summary required");
    };
    assert_eq!(fields[0].size, 14);
    assert_eq!(fields[0].filename.as_deref(), Some("image.png"));
    assert_eq!(
        recordings[0].response,
        RecordedResponse::Bytes {
            headers: Default::default(),
            status: 200,
            body_base64: "AP8qW3JlZGFjdGVkXQ==".to_owned()
        }
    );
    assert!(
        !serde_json::to_string(&recordings)
            .expect("json")
            .contains("private-upload")
    );
    assert_eq!(
        BASE64_STANDARD
            .decode("AP8qW3JlZGFjdGVkXQ==")
            .expect("base64"),
        [vec![0, 255, 42], b"[redacted]".to_vec()].concat()
    );
}

#[tokio::test]
async fn records_consumed_sse_events_and_failure_without_error_text() {
    let stream = Unimock::new((
        JsonHttpSseStreamMock::headers
            .each_call(matching!())
            .returns(JsonHttpHeaders::new()),
        JsonHttpSseStreamMock::next.stub(|calls| {
            calls
                .call(matching!())
                .answers(&|_| {
                    Ok(Some(JsonHttpSseEvent {
                        event: Some("delta".to_owned()),
                        data: "sentinel-private-key".to_owned(),
                    }))
                })
                .once()
                .then()
                .answers(&|_| {
                    Err(Error::IdleTimeout {
                        idle: Duration::from_secs(10),
                        events_received: 1,
                    }
                    .with_response_headers(JsonHttpHeaders::new()))
                });
        }),
    ));
    let stream = std::sync::Mutex::new(Some(stream));
    let upstream = Unimock::new(
        JsonHttpTransportMock::execute_sse
            .some_call(matching!(_))
            .answers_arc(Arc::new(move |_, _| {
                Ok(Box::new(
                    stream.lock().expect("lock").take().expect("one stream"),
                ))
            })),
    );
    let log: Arc<dyn RecordingLog + Send + Sync> = Arc::new(MemoryRecordingLog::default());
    let transport = RecordingTransport::new(Arc::new(upstream), log.clone());
    let mut stream = transport.execute_sse(&request()).await.expect("opens");
    assert_eq!(
        stream.next().await.expect("event").expect("one event").data,
        "sentinel-private-key"
    );
    let error = stream.next().await.expect_err("idle failure");
    assert!(matches!(
        error.cause(),
        Error::IdleTimeout {
            events_received: 1,
            ..
        }
    ));
    let recordings = log.snapshot();
    let RecordedResponse::Sse { events, end, .. } = &recordings[0].response else {
        panic!("SSE log required");
    };
    assert_eq!(events[0].data, "[redacted]");
    assert_eq!(
        end,
        &StreamEnd::Failure {
            after_events: 1,
            failure: RecordedFailure::IdleTimeout {
                idle: Duration::from_secs(10)
            }
        }
    );
}

#[tokio::test]
async fn records_rejected_stream_openings_and_continues_in_call_order() {
    let upstream = Unimock::new((
        JsonHttpTransportMock::execute_sse
            .some_call(matching!(_))
            .answers(&|_, _| {
                Err(Error::HttpStatus {
                    headers: Default::default(),
                    status: 429,
                    body: json!({"message": "sentinel-private-key"}),
                })
            }),
        JsonHttpTransportMock::execute
            .some_call(matching!(_))
            .returns(Ok(JsonHttpResponse {
                headers: Default::default(),
                status: 200,
                body: json!({"ok": true}),
            })),
    ));
    let log: Arc<dyn RecordingLog + Send + Sync> = Arc::new(MemoryRecordingLog::default());
    let transport = RecordingTransport::new(Arc::new(upstream), log.clone());
    let error = match transport.execute_sse(&request()).await {
        Err(error) => error,
        Ok(_) => panic!("must fail"),
    };
    assert!(matches!(error, Error::HttpStatus { status: 429, .. }));
    transport
        .execute(&request())
        .await
        .expect("next call succeeds");
    let recordings = log.snapshot();
    assert_eq!(recordings.len(), 2);
    assert_eq!(
        recordings[0].response,
        RecordedResponse::Json {
            headers: Default::default(),
            status: 429,
            body: json!({"message": "[redacted]"})
        }
    );
    assert_eq!(
        recordings[1].response,
        RecordedResponse::Json {
            headers: Default::default(),
            status: 200,
            body: json!({"ok": true})
        }
    );
}
