//! Ordered replay and deterministic stream failure tests.

use std::time::Duration;

use serde_json::json;

use crate::{Error, JsonHttpMethod, JsonHttpTransport};

use super::recording_tests::request;
use super::{
    RecordedEvent, RecordedFailure, RecordedResponse, Recording, Redactor, ReplayControl,
    ReplayError, ReplayTransport, StreamEnd,
};

fn recording(response: RecordedResponse) -> Recording {
    let request = request();
    Recording {
        request: Redactor::for_request(&request).request(&request),
        response,
    }
}

#[test]
fn rejects_wrong_method_and_url_without_consuming_the_recording() {
    let transport = ReplayTransport::new(vec![recording(RecordedResponse::Json {
        headers: Default::default(),
        status: 200,
        body: json!({}),
    })]);
    let mut call = request();
    call.method = JsonHttpMethod::Get;
    assert!(matches!(
        transport.next_response(&call),
        Err(ReplayError::RequestMismatch)
    ));
    call = request();
    call.url = "https://other.example.com/chat".to_owned();
    assert!(matches!(
        transport.next_response(&call),
        Err(ReplayError::RequestMismatch)
    ));
    assert_eq!(transport.remaining(), 1);
    transport
        .next_response(&request())
        .expect("matching request");
    assert!(matches!(
        transport.next_response(&request()),
        Err(ReplayError::Exhausted)
    ));
}

#[tokio::test]
async fn replays_json_bytes_and_rejected_sse_openings_in_order() {
    let transport = ReplayTransport::new(vec![
        recording(RecordedResponse::Json {
            headers: Default::default(),
            status: 201,
            body: json!({"ok": true}),
        }),
        recording(RecordedResponse::Bytes {
            headers: Default::default(),
            status: 206,
            body_base64: "AP8q".to_owned(),
        }),
        recording(RecordedResponse::Json {
            headers: Default::default(),
            status: 429,
            body: json!({"error": "rate limit"}),
        }),
    ]);
    assert_eq!(
        transport.execute(&request()).await.expect("JSON").status,
        201
    );
    assert_eq!(
        transport
            .execute_bytes(&request())
            .await
            .expect("bytes")
            .body,
        vec![0, 255, 42]
    );
    let error = match transport.execute_sse(&request()).await {
        Err(error) => error,
        Ok(_) => panic!("must fail"),
    };
    assert!(matches!(error, Error::HttpStatus { status: 429, .. }));
    assert_eq!(transport.remaining(), 0);
    assert!(matches!(
        transport.execute(&request()).await,
        Err(Error::Transport { .. })
    ));
}

#[tokio::test]
async fn replays_each_failure_class_at_the_recorded_event_count() {
    for failure in [
        RecordedFailure::Transport,
        RecordedFailure::IdleTimeout {
            idle: Duration::from_secs(10),
        },
        RecordedFailure::DeadlineExceeded {
            timeout: Duration::from_secs(60),
        },
        RecordedFailure::Decode,
    ] {
        let transport = ReplayTransport::new(vec![recording(RecordedResponse::Sse {
            headers: Default::default(),
            status: 200,
            events: vec![
                RecordedEvent {
                    event: Some("delta".to_owned()),
                    data: "partial".to_owned(),
                },
                RecordedEvent {
                    event: None,
                    data: "must not be emitted".to_owned(),
                },
            ],
            end: StreamEnd::Failure {
                after_events: 1,
                failure: failure.clone(),
            },
        })]);
        let mut stream = transport.execute_sse(&request()).await.expect("opens");
        assert_eq!(
            stream.next().await.expect("event").expect("one event").data,
            "partial"
        );
        let error = stream.next().await.expect_err("recorded failure");
        let count = match (&failure, error) {
            (
                RecordedFailure::Transport,
                Error::SseTransport {
                    events_received, ..
                },
            )
            | (
                RecordedFailure::IdleTimeout { .. },
                Error::IdleTimeout {
                    events_received, ..
                },
            )
            | (
                RecordedFailure::DeadlineExceeded { .. },
                Error::DeadlineExceeded {
                    events_received, ..
                },
            )
            | (
                RecordedFailure::Decode,
                Error::SseDecode {
                    events_received, ..
                },
            ) => events_received,
            (_, error) => panic!("wrong replay error: {error:?}"),
        };
        assert_eq!(count, 1);
        assert_eq!(stream.next().await.expect("stream ends"), None);
    }
}

#[tokio::test]
async fn replays_graceful_eof_after_every_recorded_event() {
    let transport = ReplayTransport::new(vec![recording(RecordedResponse::Sse {
        headers: Default::default(),
        status: 200,
        events: vec![RecordedEvent {
            event: None,
            data: "done".to_owned(),
        }],
        end: StreamEnd::Done,
    })]);
    let mut stream = transport.execute_sse(&request()).await.expect("opens");
    assert_eq!(
        stream.next().await.expect("event").expect("one event").data,
        "done"
    );
    assert_eq!(stream.next().await.expect("EOF"), None);
    assert_eq!(stream.next().await.expect("repeat EOF"), None);
}
