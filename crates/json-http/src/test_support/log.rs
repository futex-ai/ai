//! Shared in-memory log with reservation order independent of completion order.

use std::sync::{Mutex, MutexGuard};

use super::{
    RecordedEvent, RecordedFailure, RecordedRequest, RecordedResponse, Recording, StreamEnd,
};

/// Shared recording storage used by transport and stream wrappers.
pub trait RecordingLog: Send + Sync {
    /// Reserves a position when a call begins.
    fn begin(&self, request: RecordedRequest) -> usize;
    /// Stores the result at its original call position.
    fn response(&self, index: usize, response: RecordedResponse);
    /// Appends an observed event to a stream.
    fn event(&self, index: usize, event: RecordedEvent);
    /// Stores the observed stream failure.
    fn end(&self, index: usize, end: StreamEnd);
    /// Copies the log for serialization after calls finish.
    fn snapshot(&self) -> Vec<Recording>;
}

/// Thread-safe log that recovers its data after a poisoned lock.
#[derive(Default)]
pub struct MemoryRecordingLog {
    recordings: Mutex<Vec<Recording>>,
}

impl MemoryRecordingLog {
    fn lock(&self) -> MutexGuard<'_, Vec<Recording>> {
        self.recordings
            .lock()
            .unwrap_or_else(|poison| poison.into_inner())
    }
}

impl RecordingLog for MemoryRecordingLog {
    fn begin(&self, request: RecordedRequest) -> usize {
        let mut log = self.lock();
        let index = log.len();
        log.push(Recording {
            request,
            response: RecordedResponse::Failure {
                headers: None,
                failure: RecordedFailure::Pending,
            },
        });
        index
    }

    fn response(&self, index: usize, response: RecordedResponse) {
        if let Some(recording) = self.lock().get_mut(index) {
            recording.response = response;
        }
    }

    fn event(&self, index: usize, event: RecordedEvent) {
        if let Some(Recording {
            response: RecordedResponse::Sse { events, .. },
            ..
        }) = self.lock().get_mut(index)
        {
            events.push(event);
        }
    }

    fn end(&self, index: usize, end: StreamEnd) {
        if let Some(Recording {
            response: RecordedResponse::Sse {
                end: recorded_end, ..
            },
            ..
        }) = self.lock().get_mut(index)
        {
            *recorded_end = end;
        }
    }

    fn snapshot(&self) -> Vec<Recording> {
        self.lock().clone()
    }
}
