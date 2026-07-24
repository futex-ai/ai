//! Cryptographically secure random-byte boundary.

use std::sync::Arc;

use crate::{Error, Result};

/// Shared OAuth random-byte source.
pub type DynOAuthRandom = Arc<dyn OAuthRandom>;

#[cfg_attr(
    any(test, doctest, feature = "test-support"),
    unimock::unimock(api = OAuthRandomMock)
)]
/// Supplies cryptographically secure bytes for state and PKCE.
pub trait OAuthRandom: Send + Sync {
    /// Returns exactly `length` random bytes.
    fn bytes(&self, length: usize) -> Result<Vec<u8>>;
}

#[derive(Clone, Copy, Debug, Default)]
/// Production operating-system random-byte source.
pub struct SystemOAuthRandom;

impl OAuthRandom for SystemOAuthRandom {
    fn bytes(&self, length: usize) -> Result<Vec<u8>> {
        let mut bytes = vec![0_u8; length];
        if getrandom::fill(&mut bytes).is_err() {
            return Err(Error::Random);
        }
        Ok(bytes)
    }
}
