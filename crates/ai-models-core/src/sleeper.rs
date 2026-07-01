//! Sleeper boundary used by retry wrappers.

use std::{sync::Arc, time::Duration};

use async_trait::async_trait;

/// Shared dynamic sleeper alias.
pub type DynSleeper = Arc<dyn Sleeper>;

#[cfg_attr(any(test, doctest), unimock::unimock(api = SleeperMock))]
#[async_trait]
/// Async sleep boundary used by retrying wrappers.
pub trait Sleeper: Send + Sync {
    /// Sleeps for the provided duration.
    async fn sleep(&self, duration: Duration);
}

#[derive(Clone, Debug, Default)]
/// Tokio-backed sleeper implementation for production use.
pub struct TokioSleeper;

#[async_trait]
impl Sleeper for TokioSleeper {
    async fn sleep(&self, duration: Duration) {
        tokio::time::sleep(duration).await;
    }
}
