//! Reqwest SSE response stream with idle and overall deadline enforcement.

use std::time::Duration;

use async_trait::async_trait;
use tokio::time::{Instant, timeout};

use crate::{Error, JsonHttpSseDecoder, JsonHttpSseEvent, JsonHttpSseStream, Result};

#[derive(Clone, Copy)]
pub(crate) enum TimeoutKind {
    Deadline,
    Idle(Duration),
}

pub(crate) struct SseTimer {
    started: Instant,
    last_event: Instant,
    timeout: Duration,
    idle_timeout: Option<Duration>,
}

impl SseTimer {
    pub(crate) fn new(timeout: Duration, idle_timeout: Option<Duration>) -> Self {
        let started = Instant::now();
        Self {
            started,
            last_event: started,
            timeout,
            idle_timeout,
        }
    }

    pub(crate) fn next_wait(&self) -> (Duration, TimeoutKind) {
        let deadline = self.timeout.saturating_sub(self.started.elapsed());
        match self.idle_timeout {
            Some(idle) => {
                let idle_remaining = idle.saturating_sub(self.last_event.elapsed());
                if idle_remaining < deadline {
                    (idle_remaining, TimeoutKind::Idle(idle))
                } else {
                    (deadline, TimeoutKind::Deadline)
                }
            }
            None => (deadline, TimeoutKind::Deadline),
        }
    }

    pub(crate) fn error(&self, kind: TimeoutKind, events_received: u64) -> Error {
        match kind {
            TimeoutKind::Deadline => Error::DeadlineExceeded {
                timeout: self.timeout,
                events_received,
            },
            TimeoutKind::Idle(idle) => Error::IdleTimeout {
                idle,
                events_received,
            },
        }
    }

    fn record_event(&mut self) {
        self.last_event = Instant::now();
    }
}

pub(crate) struct ReqwestJsonHttpSseStream {
    response: reqwest::Response,
    decoder: JsonHttpSseDecoder,
    timer: SseTimer,
    events_received: u64,
    eof: bool,
}

impl ReqwestJsonHttpSseStream {
    pub(crate) fn new(response: reqwest::Response, timer: SseTimer) -> Self {
        Self {
            response,
            decoder: JsonHttpSseDecoder::new(),
            timer,
            events_received: 0,
            eof: false,
        }
    }
}

#[async_trait]
impl JsonHttpSseStream for ReqwestJsonHttpSseStream {
    async fn next(&mut self) -> Result<Option<JsonHttpSseEvent>> {
        loop {
            let event = match self.decoder.next_event(self.eof) {
                Ok(event) => event,
                Err(source) => {
                    return Err(Error::SseDecode {
                        events_received: self.events_received,
                        source,
                    });
                }
            };
            if let Some(event) = event {
                self.events_received = self.events_received.saturating_add(1);
                self.timer.record_event();
                return Ok(Some(event));
            }
            if self.eof {
                return Ok(None);
            }

            let (wait, kind) = self.timer.next_wait();
            if wait.is_zero() {
                return Err(self.timer.error(kind, self.events_received));
            }
            match timeout(wait, self.response.chunk()).await {
                Ok(Ok(Some(bytes))) => self.decoder.push(&bytes),
                Ok(Ok(None)) => self.eof = true,
                Ok(Err(source)) => {
                    return Err(Error::SseTransport {
                        events_received: self.events_received,
                        source,
                    });
                }
                Err(_) => return Err(self.timer.error(kind, self.events_received)),
            }
        }
    }
}
