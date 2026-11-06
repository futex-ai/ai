//! Bounds and retry bases for one model call.

use std::{num::NonZeroUsize, time::Duration};

/// Default retry bases: `100ms`, then `250ms`.
pub const STANDARD_TRANSIENT_RETRY_DELAYS: [Duration; 2] =
    [Duration::from_millis(100), Duration::from_millis(250)];

#[derive(Clone, Debug, Eq, PartialEq)]
/// Retry limits shared by opening and reading the inner stream.
pub struct RetryConfig {
    /// Maximum attempts, including the first invocation.
    pub max_attempts: NonZeroUsize,
    /// Base for each retry. An exhausted schedule stops retrying.
    pub base_delays: Vec<Duration>,
    /// Longest accepted provider retry-after value.
    pub max_wait: Duration,
}

impl Default for RetryConfig {
    fn default() -> Self {
        Self {
            max_attempts: NonZeroUsize::new(3).unwrap_or(NonZeroUsize::MIN),
            base_delays: STANDARD_TRANSIENT_RETRY_DELAYS.to_vec(),
            max_wait: Duration::from_secs(30),
        }
    }
}
