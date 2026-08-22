//! Tests for pure incremental SSE framing.

use crate::{JsonHttpSseDecodeError, JsonHttpSseDecoder, JsonHttpSseEvent};

#[test]
fn decodes_named_event_split_across_chunks() {
    let mut decoder = JsonHttpSseDecoder::new();
    decoder.push(b"event: content");
    decoder.push(b"_delta\r");
    assert_eq!(decoder.next_event(false).unwrap(), None);

    decoder.push(b"\ndata: hel");
    decoder.push(b"lo\r\n\r\n");

    assert_eq!(
        decoder.next_event(false).unwrap(),
        Some(JsonHttpSseEvent {
            event: Some("content_delta".to_owned()),
            data: "hello".to_owned(),
        })
    );
}

#[test]
fn accepts_every_whatwg_event_boundary() {
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
        let mut decoder = JsonHttpSseDecoder::new();
        decoder.push(b"data: value");
        decoder.push(boundary);

        assert_eq!(
            decoder.next_event(false).unwrap(),
            Some(JsonHttpSseEvent {
                event: None,
                data: "value".to_owned(),
            }),
            "boundary {boundary:?}"
        );
    }
}

#[test]
fn joins_multiline_and_colonless_data_without_trimming_content() {
    let mut decoder = JsonHttpSseDecoder::new();
    decoder.push(
        b": heartbeat\nid: 7\nretry: 100\nunknown: ignored\n\
          event:first\nevent: final\ndata:first\ndata\ndata:  two spaces \n\n",
    );

    assert_eq!(
        decoder.next_event(false).unwrap(),
        Some(JsonHttpSseEvent {
            event: Some("final".to_owned()),
            data: "first\n\n two spaces ".to_owned(),
        })
    );
}

#[test]
fn skips_blocks_without_data_and_decodes_multiple_events() {
    let mut decoder = JsonHttpSseDecoder::new();
    decoder.push(b": comment\nevent: ping\n\nid: 1\n\ndata: one\n\ndata: two\n\n");

    assert_eq!(
        decoder.next_event(false).unwrap(),
        Some(JsonHttpSseEvent {
            event: None,
            data: "one".to_owned(),
        })
    );
    assert_eq!(
        decoder.next_event(false).unwrap(),
        Some(JsonHttpSseEvent {
            event: None,
            data: "two".to_owned(),
        })
    );
    assert_eq!(decoder.next_event(false).unwrap(), None);
}

#[test]
fn dispatches_pending_data_at_eof() {
    let mut decoder = JsonHttpSseDecoder::new();
    decoder.push(b"event: done\rdata: final");

    assert_eq!(
        decoder.next_event(true).unwrap(),
        Some(JsonHttpSseEvent {
            event: Some("done".to_owned()),
            data: "final".to_owned(),
        })
    );
    assert_eq!(decoder.next_event(true).unwrap(), None);
}

#[test]
fn preserves_utf8_split_across_input_chunks() {
    let mut decoder = JsonHttpSseDecoder::new();
    let event = "data: thinking 🧠\n\n".as_bytes();
    let split = event
        .windows(4)
        .position(|window| window == "🧠".as_bytes())
        .expect("brain bytes should be present");

    decoder.push(&event[..split + 2]);
    decoder.push(&event[split + 2..]);

    assert_eq!(
        decoder.next_event(false).unwrap(),
        Some(JsonHttpSseEvent {
            event: None,
            data: "thinking 🧠".to_owned(),
        })
    );
}

#[test]
fn decodes_an_event_at_every_possible_chunk_split() {
    let bytes = "event: delta\r\ndata: part 🧠\r\ndata: two\r\n\r\n".as_bytes();

    for split in 0..=bytes.len() {
        let mut decoder = JsonHttpSseDecoder::new();
        decoder.push(&bytes[..split]);
        decoder.push(&bytes[split..]);

        assert_eq!(
            decoder.next_event(false).unwrap(),
            Some(JsonHttpSseEvent {
                event: Some("delta".to_owned()),
                data: "part 🧠\ntwo".to_owned(),
            }),
            "split at byte {split}"
        );
    }
}

#[test]
fn rejects_invalid_utf8_in_a_complete_event() {
    let mut decoder = JsonHttpSseDecoder::new();
    decoder.push(b"data: \xff\n\n");

    assert!(matches!(
        decoder.next_event(false).unwrap_err(),
        JsonHttpSseDecodeError::InvalidUtf8 { .. }
    ));
}
