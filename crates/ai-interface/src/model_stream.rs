//! Pull-based model stream contract.

use async_trait::async_trait;

use crate::{ModelEvent, ModelResult};

/// Owned stream returned by a model or policy layer.
pub type DynModelEventStream = Box<dyn ModelEventStream>;

#[cfg_attr(
    any(test, doctest, feature = "test-support"),
    unimock::unimock(api = ModelEventStreamMock)
)]
#[async_trait]
/// Ordered model events with single-owner consumption and cancellation on drop.
pub trait ModelEventStream: Send {
    /// Returns one event, or `None` after the finish event or an earlier failure.
    ///
    /// After this returns `None` or an error, every later call returns `None`.
    async fn next(&mut self) -> ModelResult<Option<ModelEvent>>;
}
