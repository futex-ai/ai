//! Pure incremental Server-Sent Events framing.

use std::str::Utf8Error;

use async_trait::async_trait;
use thiserror::Error;

use crate::Result;

/// Owned dynamic SSE stream returned by JSON HTTP transports.
pub type DynJsonHttpSseStream = Box<dyn JsonHttpSseStream>;

#[cfg_attr(
    any(test, doctest, feature = "test-support"),
    unimock::unimock(api = JsonHttpSseStreamMock)
)]
#[async_trait]
/// Pull-based stream of decoded Server-Sent Events.
pub trait JsonHttpSseStream: Send {
    /// Returns the next event or `None` after a graceful response-body EOF.
    async fn next(&mut self) -> Result<Option<JsonHttpSseEvent>>;
}

#[derive(Clone, Debug, Eq, PartialEq)]
/// One decoded Server-Sent Event.
pub struct JsonHttpSseEvent {
    /// Optional provider event name from the final `event` field.
    pub event: Option<String>,
    /// Joined payload from every `data` field.
    pub data: String,
}

#[derive(Clone, Copy, Debug, Error)]
/// Errors returned by incremental SSE framing.
pub enum JsonHttpSseDecodeError {
    /// One complete event was not valid UTF-8.
    #[error("[json_http/sse] SSE event was not valid UTF-8: {source}")]
    InvalidUtf8 {
        /// Underlying UTF-8 validation failure.
        source: Utf8Error,
    },
}

#[derive(Debug, Default)]
/// Pure decoder that accepts arbitrary response-body byte chunks.
pub struct JsonHttpSseDecoder {
    buffer: Vec<u8>,
}

impl JsonHttpSseDecoder {
    /// Creates an empty incremental decoder.
    pub fn new() -> Self {
        Self::default()
    }

    /// Appends response bytes without requiring an event or UTF-8 boundary.
    pub fn push(&mut self, bytes: &[u8]) {
        self.buffer.extend_from_slice(bytes);
    }

    /// Returns the next complete event, optionally dispatching pending data at EOF.
    pub fn next_event(
        &mut self,
        eof: bool,
    ) -> std::result::Result<Option<JsonHttpSseEvent>, JsonHttpSseDecodeError> {
        loop {
            let event = if let Some(end) = event_end(&self.buffer) {
                let bytes = self.buffer.drain(..end.consumed).collect::<Vec<_>>();
                Some(bytes[..end.payload].to_vec())
            } else if eof && !self.buffer.is_empty() {
                Some(std::mem::take(&mut self.buffer))
            } else {
                None
            };
            let Some(event) = event else {
                return Ok(None);
            };
            if let Some(event) = decode_event(&event)? {
                return Ok(Some(event));
            }
        }
    }
}

struct EventEnd {
    payload: usize,
    consumed: usize,
}

fn event_end(buffer: &[u8]) -> Option<EventEnd> {
    let mut line_start = 0;
    let mut index = 0;
    while index < buffer.len() {
        let terminator_len = match buffer[index] {
            b'\n' => 1,
            b'\r' if buffer.get(index + 1) == Some(&b'\n') => 2,
            b'\r' => 1,
            _ => {
                index += 1;
                continue;
            }
        };
        if index == line_start {
            return Some(EventEnd {
                payload: line_start,
                consumed: index + terminator_len,
            });
        }
        index += terminator_len;
        line_start = index;
    }
    None
}

fn decode_event(
    bytes: &[u8],
) -> std::result::Result<Option<JsonHttpSseEvent>, JsonHttpSseDecodeError> {
    let text = match std::str::from_utf8(bytes) {
        Ok(text) => text,
        Err(source) => return Err(JsonHttpSseDecodeError::InvalidUtf8 { source }),
    };
    let normalized = text.replace("\r\n", "\n").replace('\r', "\n");
    let mut event = None;
    let mut data = Vec::new();
    let mut data_seen = false;

    for line in normalized.split('\n') {
        if line.starts_with(':') {
            continue;
        }
        let (field, value) = line.split_once(':').unwrap_or((line, ""));
        let value = value.strip_prefix(' ').unwrap_or(value);
        match field {
            "data" => {
                data_seen = true;
                data.push(value);
            }
            "event" => event = (!value.is_empty()).then(|| value.to_owned()),
            _ => {}
        }
    }

    Ok(data_seen.then(|| JsonHttpSseEvent {
        event,
        data: data.join("\n"),
    }))
}
