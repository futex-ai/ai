//! Incremental SSE decoding tests.

use serde_json::json;

use crate::{Error, transport::sse::SseDecoder};

#[test]
fn yields_completed_events_before_eof_across_split_chunks() {
    let mut decoder = SseDecoder::new(1024);
    decoder.push(b"event: message\ndata: {\"json").unwrap();
    assert_eq!(decoder.next_message(false).unwrap(), None);

    decoder
        .push(b"rpc\":\"2.0\",\"id\":1,\"result\":{}}\n\nstill-open")
        .unwrap();
    assert_eq!(
        decoder.next_message(false).unwrap(),
        Some(json!({"jsonrpc":"2.0","id":1,"result":{}}))
    );
    assert_eq!(decoder.next_message(false).unwrap(), None);
}

#[test]
fn joins_multiline_data_and_ignores_other_fields() {
    let mut decoder = SseDecoder::new(1024);
    decoder
        .push(b"id: 7\nretry: 100\ndata: {\"value\":\ndata: 42}\n\n")
        .unwrap();

    assert_eq!(
        decoder.next_message(false).unwrap(),
        Some(json!({"value": 42}))
    );
}

#[test]
fn enforces_the_cumulative_raw_byte_limit() {
    let mut decoder = SseDecoder::new(10);
    decoder.push(b"data: 1\n").unwrap();
    let error = decoder.push(b"\nmore").unwrap_err();

    assert!(matches!(error, Error::ResponseTooLarge { limit_bytes: 10 }));
}

#[test]
fn dispatches_a_final_complete_event_at_eof() {
    let mut decoder = SseDecoder::new(1024);
    decoder.push(b"data: {\"done\":true}").unwrap();

    assert_eq!(
        decoder.next_message(true).unwrap(),
        Some(json!({"done": true}))
    );
}
