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

#[test]
fn dispatches_a_cr_framed_event_before_eof() {
    let mut decoder = SseDecoder::new(1024);
    decoder.push(b"data: {\"a\":1}\r\r").unwrap();

    assert_eq!(decoder.next_message(false).unwrap(), Some(json!({"a": 1})));
}

#[test]
fn joins_cr_terminated_data_lines_at_eof() {
    let mut decoder = SseDecoder::new(1024);
    decoder.push(b"data: {\"a\":\rdata: 1}").unwrap();

    assert_eq!(decoder.next_message(true).unwrap(), Some(json!({"a": 1})));
}

#[test]
fn accepts_every_valid_event_boundary_encoding() {
    let boundaries: &[&[u8]] = &[
        b"\n\n",
        b"\n\r",
        b"\n\r\n",
        b"\r\r",
        b"\r\r\n",
        b"\r\n\n",
        b"\r\n\r",
        b"\r\n\r\n",
    ];

    for boundary in boundaries {
        let mut decoder = SseDecoder::new(1024);
        decoder.push(b"data: 1").unwrap();
        decoder.push(boundary).unwrap();

        assert_eq!(
            decoder.next_message(false).unwrap(),
            Some(json!(1)),
            "boundary {boundary:?}"
        );
    }
}

#[test]
fn trailing_cr_blank_line_dispatches_without_another_byte() {
    let mut decoder = SseDecoder::new(1024);
    decoder.push(b"data: {\"a\":1}\n\r").unwrap();

    assert_eq!(decoder.next_message(false).unwrap(), Some(json!({"a": 1})));
}

#[test]
fn lf_after_a_cr_completed_event_is_an_empty_no_op() {
    let mut decoder = SseDecoder::new(1024);
    decoder.push(b"data: {\"a\":1}\n\r").unwrap();
    assert_eq!(decoder.next_message(false).unwrap(), Some(json!({"a": 1})));

    decoder.push(b"\n").unwrap();
    assert_eq!(decoder.next_message(false).unwrap(), None);
    decoder.push(b"data: {\"b\":2}\n\n").unwrap();

    assert_eq!(decoder.next_message(false).unwrap(), Some(json!({"b": 2})));
}

#[test]
fn decodes_multiple_events_with_mixed_framing() {
    let mut decoder = SseDecoder::new(1024);
    decoder
        .push(b"data: 1\r\rdata: 2\r\n\r\ndata: 3\n\n")
        .unwrap();

    assert_eq!(decoder.next_message(false).unwrap(), Some(json!(1)));
    assert_eq!(decoder.next_message(false).unwrap(), Some(json!(2)));
    assert_eq!(decoder.next_message(false).unwrap(), Some(json!(3)));
    assert_eq!(decoder.next_message(false).unwrap(), None);
}

#[test]
fn one_crlf_pair_is_not_a_blank_line() {
    let mut decoder = SseDecoder::new(1024);
    decoder.push(b"data: {\"v\":\r\ndata: 42}\n\n").unwrap();

    assert_eq!(decoder.next_message(false).unwrap(), Some(json!({"v": 42})));
    assert_eq!(decoder.next_message(false).unwrap(), None);
}

#[test]
fn crlf_split_across_chunks_remains_one_line_ending() {
    let mut decoder = SseDecoder::new(1024);
    decoder.push(b"data: {\"v\":\r").unwrap();
    assert_eq!(decoder.next_message(false).unwrap(), None);

    decoder.push(b"\ndata: 42}\n\n").unwrap();

    assert_eq!(decoder.next_message(false).unwrap(), Some(json!({"v": 42})));
    assert_eq!(decoder.next_message(false).unwrap(), None);
}
