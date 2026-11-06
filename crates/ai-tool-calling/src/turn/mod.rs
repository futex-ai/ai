//! Turn outcomes, execution records, and checkpoint hooks.

mod batch;
mod event_copy;
mod failures;
mod observer;
mod pending;
mod response;
pub(crate) mod stream;
mod tool_dispatch;
mod types;

pub use types::{
    ModelResponseCheckpoint, NoopModelResponseCheckpoint, NoopTurnCheckpoint, RunOutcome,
    ToolExecutionRecord, TurnCheckpoint,
};

#[cfg(any(test, doctest, feature = "test-support"))]
pub use types::{ModelResponseCheckpointMock, TurnCheckpointMock};
