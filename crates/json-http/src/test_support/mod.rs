//! Credential-free exchange recording and ordered transport replay.

mod failure;
mod headers;
mod log;
mod optional_headers;
mod recording;
mod redaction;
mod replay;
mod types;

pub use failure::RecordedFailure;
pub use log::{MemoryRecordingLog, RecordingLog};
pub use recording::RecordingTransport;
pub use redaction::Redactor;
pub use replay::{ReplayControl, ReplayError, ReplayTransport};
pub use types::{
    MultipartSummary, RecordedBody, RecordedEvent, RecordedMethod, RecordedRequest,
    RecordedResponse, Recording, StreamEnd,
};

#[cfg(test)]
#[path = "_tests_/headers_tests.rs"]
mod headers_tests;
#[cfg(test)]
#[path = "_tests_/recording_tests.rs"]
mod recording_tests;
#[cfg(test)]
#[path = "_tests_/replay_tests.rs"]
mod replay_tests;
#[cfg(test)]
#[path = "_tests_/response_context_tests.rs"]
mod response_context_tests;
