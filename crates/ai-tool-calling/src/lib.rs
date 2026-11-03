//! Generic in-memory tool-calling runtime.

#![warn(unreachable_pub)]

mod agent;
mod catalog;
mod error;
mod intrinsic;
mod output_store;
mod policy;
mod runtime;
mod tool_output;
mod turn;
mod typed_tool;

pub use agent::{Agent, AgentEvent, AgentEventStream, AgentRequest, DynAgentEventStream};
#[cfg(any(test, doctest, feature = "test-support"))]
pub use agent::{AgentEventStreamMock, AgentMock};
pub use error::{Error, Result};
pub use output_store::{
    DynToolOutputStore, InMemoryToolOutputStore, ToolOutputStore, ToolOutputStoreError,
    ToolOutputStoreReadRequest, ToolOutputStoreResult, ToolOutputStoreWindow,
    ToolOutputWriteRequest, ToolOutputWriteResult,
};
pub use policy::{
    DEFAULT_AGGREGATE_LIMIT_BYTES, DEFAULT_INLINE_LIMIT_BYTES, DEFAULT_PER_OUTPUT_LIMIT_BYTES,
    DEFAULT_READ_LIMIT_BYTES, ToolOutputPolicy, ToolOutputPolicyError, ToolOutputPolicyLimits,
    ToolOutputPolicyResult,
};
pub use runtime::ToolCallingRuntime;
pub use turn::ToolExecutionRecord;
pub use turn::{
    ModelResponseCheckpoint, NoopModelResponseCheckpoint, NoopTurnCheckpoint, RunOutcome,
    TurnCheckpoint,
};
pub use typed_tool::{TypedTool, TypedToolAdapter};

#[cfg(any(test, doctest, feature = "test-support"))]
pub use output_store::ToolOutputStoreMock;
#[cfg(test)]
pub(crate) use output_store::{OutputIdGenerationError, ToolOutputIdGeneratorMock};
#[cfg(any(test, doctest, feature = "test-support"))]
pub use turn::{ModelResponseCheckpointMock, TurnCheckpointMock};

#[cfg(test)]
#[path = "_tests_/mod.rs"]
mod tests;
