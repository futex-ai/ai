//! Wall-clock dependency for provider HTTP-date retry hints.

use std::time::SystemTime;

#[cfg_attr(any(test, doctest, feature = "test-support"), unimock::unimock(api = ClockMock))]
/// Supplies wall time when a response contains an absolute retry date.
pub trait Clock: Send + Sync {
    /// Returns the current wall time.
    fn now(&self) -> SystemTime;
}

#[derive(Clone, Copy, Debug, Default)]
/// Production wall clock for response metadata parsing.
pub struct SystemClock;

impl Clock for SystemClock {
    fn now(&self) -> SystemTime {
        SystemTime::now()
    }
}
