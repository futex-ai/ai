//! Tool output storage boundary.

mod error;
mod id_generator;
mod memory;
mod types;
mod windowing;

use std::sync::Arc;

use async_trait::async_trait;

pub use error::{ToolOutputStoreError, ToolOutputStoreResult};
pub use memory::InMemoryToolOutputStore;
pub use types::{
    ToolOutputStoreReadRequest, ToolOutputStoreWindow, ToolOutputWriteRequest,
    ToolOutputWriteResult,
};

pub(crate) use id_generator::{DynToolOutputIdGenerator, SystemToolOutputIdGenerator};
#[cfg(test)]
pub(crate) use id_generator::{OutputIdGenerationError, ToolOutputIdGeneratorMock};

#[cfg_attr(
    any(test, doctest, feature = "test-support"),
    unimock::unimock(api = ToolOutputStoreMock)
)]
#[async_trait]
/// Async storage boundary for retained windowed tool outputs.
pub trait ToolOutputStore: Send + Sync {
    /// Stores complete serialized output and returns its first window.
    async fn write(
        &self,
        request: ToolOutputWriteRequest,
    ) -> ToolOutputStoreResult<ToolOutputWriteResult>;

    /// Reads a UTF-8-safe byte window for a previously stored output.
    async fn read(
        &self,
        request: ToolOutputStoreReadRequest,
    ) -> ToolOutputStoreResult<ToolOutputStoreWindow>;
}

/// Shared dynamic tool output store alias.
pub type DynToolOutputStore = Arc<dyn ToolOutputStore + Send + Sync>;
