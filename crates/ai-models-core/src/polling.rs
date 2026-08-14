//! Runtime boundary for deadline-aware polling adapters.

use std::{
    sync::Arc,
    time::{Duration, Instant},
};

use async_trait::async_trait;

/// Shared dynamic polling runtime alias.
pub type DynPollingRuntime = Arc<dyn PollingRuntime>;

#[cfg_attr(
    any(test, doctest, feature = "test-support"),
    unimock::unimock(api = PollingRuntimeMock)
)]
#[async_trait]
/// Clock and async sleep boundary used by long-running provider operations.
pub trait PollingRuntime: Send + Sync {
    /// Returns the current monotonic instant.
    fn now(&self) -> Instant;

    /// Sleeps for the provided duration.
    async fn sleep(&self, duration: Duration);
}

/// Tokio-backed polling runtime for production use.
#[derive(Clone, Debug, Default)]
pub struct TokioPollingRuntime;

#[async_trait]
impl PollingRuntime for TokioPollingRuntime {
    fn now(&self) -> Instant {
        Instant::now()
    }

    async fn sleep(&self, duration: Duration) {
        tokio::time::sleep(duration).await;
    }
}
