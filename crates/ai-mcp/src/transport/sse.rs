//! Incremental, size-bounded SSE event decoding.

use std::pin::Pin;

use async_trait::async_trait;
use bytes::Bytes;
use futures_util::{Stream, StreamExt};
use serde_json::Value;

use crate::{Error, Result};

use super::McpEventStream;

type ResponseByteStream = Pin<Box<dyn Stream<Item = reqwest::Result<Bytes>> + Send>>;

pub(crate) struct SseDecoder {
    buffer: Vec<u8>,
    bytes_received: usize,
    limit_bytes: usize,
}

impl SseDecoder {
    pub(crate) fn new(limit_bytes: usize) -> Self {
        Self {
            buffer: Vec::new(),
            bytes_received: 0,
            limit_bytes,
        }
    }

    pub(crate) fn push(&mut self, bytes: &[u8]) -> Result<()> {
        self.bytes_received = self.bytes_received.saturating_add(bytes.len());
        if self.bytes_received > self.limit_bytes {
            return Err(Error::ResponseTooLarge {
                limit_bytes: self.limit_bytes,
            });
        }
        self.buffer.extend_from_slice(bytes);
        Ok(())
    }

    pub(crate) fn next_message(&mut self, eof: bool) -> Result<Option<Value>> {
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
            if let Some(message) = decode_event(&event)? {
                return Ok(Some(message));
            }
        }
    }
}

struct EventEnd {
    payload: usize,
    consumed: usize,
}

/// Finds the first blank line using every WHATWG line-ending form.
///
/// A trailing CR can complete an empty line immediately because bytes are
/// drained only for a complete event. If a later LF completes that CRLF pair,
/// it forms a no-data empty event and is skipped. A CR ending a non-empty line
/// remains buffered, so a later LF is seen contiguously on the next scan.
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

fn decode_event(bytes: &[u8]) -> Result<Option<Value>> {
    let text = match std::str::from_utf8(bytes) {
        Ok(text) => text,
        Err(source) => return Err(Error::transport(&source)),
    };
    let normalized = text.replace("\r\n", "\n").replace('\r', "\n");
    let mut data = Vec::new();
    for line in normalized.split('\n') {
        let Some((field, value)) = line.split_once(':') else {
            continue;
        };
        if field == "data" {
            data.push(value.strip_prefix(' ').unwrap_or(value));
        }
    }
    if data.is_empty() {
        return Ok(None);
    }
    let joined = data.join("\n");
    match serde_json::from_str(&joined) {
        Ok(value) => Ok(Some(value)),
        Err(source) => Err(Error::deserialize("SSE event", source)),
    }
}

pub(crate) struct ReqwestEventStream {
    stream: ResponseByteStream,
    decoder: SseDecoder,
    eof: bool,
}

impl ReqwestEventStream {
    pub(crate) fn new(
        stream: impl Stream<Item = reqwest::Result<Bytes>> + Send + 'static,
        limit_bytes: usize,
    ) -> Self {
        Self {
            stream: Box::pin(stream),
            decoder: SseDecoder::new(limit_bytes),
            eof: false,
        }
    }
}

#[async_trait]
impl McpEventStream for ReqwestEventStream {
    async fn next_message(&mut self) -> Result<Option<Value>> {
        loop {
            if let Some(message) = self.decoder.next_message(self.eof)? {
                return Ok(Some(message));
            }
            if self.eof {
                return Ok(None);
            }
            match self.stream.next().await {
                Some(Ok(bytes)) => self.decoder.push(&bytes)?,
                Some(Err(source)) => return Err(Error::transport(&source)),
                None => self.eof = true,
            }
        }
    }
}
