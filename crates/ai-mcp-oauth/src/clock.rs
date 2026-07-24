//! Clock boundary for deterministic expiry and cache behavior.

use std::sync::Arc;

use crate::Result;

/// Shared OAuth clock implementation.
pub type DynOAuthClock = Arc<dyn OAuthClock>;

#[cfg_attr(
    any(test, doctest, feature = "test-support"),
    unimock::unimock(api = OAuthClockMock)
)]
/// Supplies UNIX time without ambient clock access in protocol services.
pub trait OAuthClock: Send + Sync {
    /// Returns whole seconds since the UNIX epoch.
    fn now_unix_seconds(&self) -> Result<u64>;
}

#[derive(Clone, Copy, Debug, Default)]
/// Production system-clock implementation.
pub struct SystemOAuthClock;

impl OAuthClock for SystemOAuthClock {
    fn now_unix_seconds(&self) -> Result<u64> {
        let elapsed = match std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH) {
            Ok(elapsed) => elapsed,
            Err(_) => return Err(crate::Error::Clock),
        };
        Ok(elapsed.as_secs())
    }
}
