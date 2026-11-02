//! Injectable randomness for bounded retry jitter.

use std::{sync::Arc, time::Duration};

/// Shared retry randomness boundary.
pub type DynRetryRandom = Arc<dyn RetryRandom>;

#[cfg_attr(any(test, feature = "test-support"), unimock::unimock(api = RetryRandomMock))]
/// Supplies an inclusive random duration within the requested bounds.
pub trait RetryRandom: Send + Sync {
    /// Chooses a duration from the inclusive minimum and maximum.
    fn between(&self, minimum: Duration, maximum: Duration) -> Duration;
}

#[derive(Clone, Debug, Default)]
/// System-backed random duration generator for retry jitter.
pub struct SystemRetryRandom;

impl RetryRandom for SystemRetryRandom {
    fn between(&self, minimum: Duration, maximum: Duration) -> Duration {
        let nanos = rand::random_range(minimum.as_nanos()..=maximum.as_nanos());
        Duration::new(
            (nanos / 1_000_000_000) as u64,
            (nanos % 1_000_000_000) as u32,
        )
    }
}
