//! Generic in-memory tool-calling runtime.

#![warn(unreachable_pub)]

mod error;
mod runtime;
mod turn;

pub use error::{Error, Result};
pub use runtime::ToolCallingRuntime;
pub use turn::ToolExecutionRecord;
pub use turn::{
    ActiveTurn, ModelResponseCheckpoint, NoopModelResponseCheckpoint, NoopTurnCheckpoint,
    RunOutcome, StepOutcome, Turn, TurnCheckpoint,
};

#[cfg(any(test, doctest))]
pub use turn::TurnCheckpointMock;

#[cfg(test)]
#[path = "_tests_/mod.rs"]
mod tests;
