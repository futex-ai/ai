//! Incremental model completion events and their observer boundary.

use std::{fmt, sync::Arc};

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[non_exhaustive]
/// Ordered public progress emitted while a model completion is running.
pub enum ModelCompletionEvent {
    /// Newly generated user-visible assistant text.
    AssistantTextDelta {
        /// Exact append-only text fragment emitted by the provider.
        delta: String,
    },
    /// Newly generated provider-exposed reasoning or thinking text.
    ReasoningTextDelta {
        /// Exact append-only reasoning fragment emitted by the provider.
        delta: String,
    },
    /// The prior fallback attempt failed and its emitted text must be discarded.
    AttemptRestarted,
}

#[cfg_attr(
    any(test, doctest, feature = "test-support"),
    unimock::unimock(api = ModelCompletionEventSinkMock)
)]
#[async_trait]
/// Infallible observer for ordered model completion events.
pub trait ModelCompletionEventSink: Send + Sync {
    /// Observes one event before generation continues to the next event.
    async fn emit(&self, event: ModelCompletionEvent);
}

impl fmt::Debug for dyn ModelCompletionEventSink + '_ {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("ModelCompletionEventSink")
            .finish_non_exhaustive()
    }
}

/// Shared dynamic model completion event sink alias.
pub type DynModelCompletionEventSink = Arc<dyn ModelCompletionEventSink>;

#[derive(Clone, Copy, Debug, Default)]
/// Completion event sink that intentionally ignores every event.
pub struct NoopModelCompletionEventSink;

#[async_trait]
impl ModelCompletionEventSink for NoopModelCompletionEventSink {
    async fn emit(&self, _event: ModelCompletionEvent) {}
}
