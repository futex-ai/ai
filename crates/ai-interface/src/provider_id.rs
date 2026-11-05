//! Stable provider names independent of the installed provider adapters.

use std::borrow::Cow;
use std::fmt::{Display, Formatter};

use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(transparent)]
/// Stable lowercase provider identity used by configuration and replay state.
pub struct ProviderId(Cow<'static, str>);

impl ProviderId {
    /// Builds an identity for a provider-owned constant.
    pub const fn new(value: &'static str) -> Self {
        Self(Cow::Borrowed(value))
    }

    /// Returns the stable provider name.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl From<String> for ProviderId {
    fn from(value: String) -> Self {
        Self(Cow::Owned(value))
    }
}

impl Display for ProviderId {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(self.as_str())
    }
}

/// Provider identity of the built-in development and test model.
pub const MOCK_PROVIDER: ProviderId = ProviderId::new("mock");

#[cfg(test)]
#[path = "_tests_/provider_id_tests.rs"]
mod provider_id_tests;
