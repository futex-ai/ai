//! Auth hook types for JSON HTTP requests.

use std::{collections::BTreeMap, fmt, sync::Arc};

use async_trait::async_trait;

use crate::{Result, request_headers::insert};

/// Shared dynamic auth hook alias.
pub type DynJsonHttpAuth = Arc<dyn JsonHttpAuth>;

#[cfg_attr(
    any(test, doctest, feature = "test-support"),
    unimock::unimock(api = JsonHttpAuthMock)
)]
#[async_trait]
/// Applies request headers before a JSON HTTP request is dispatched.
pub trait JsonHttpAuth: Send + Sync {
    /// Mutates the outgoing request headers in place.
    async fn apply_headers(&self, headers: &mut BTreeMap<String, String>) -> Result<()>;
}

#[derive(Clone, Default, Eq, PartialEq)]
/// Simple auth hook that applies a fixed set of headers.
pub struct StaticHeaderAuth {
    headers: BTreeMap<String, String>,
}

impl StaticHeaderAuth {
    /// Builds a static auth hook from the provided header map.
    pub fn new(headers: BTreeMap<String, String>) -> Self {
        Self { headers }
    }

    /// Builds a bearer-token auth hook for Authorization headers.
    pub fn bearer_token(token: impl Into<String>) -> Self {
        Self::new(BTreeMap::from([(
            "Authorization".to_owned(),
            format!("Bearer {}", token.into()),
        )]))
    }
}

#[async_trait]
impl JsonHttpAuth for StaticHeaderAuth {
    async fn apply_headers(&self, headers: &mut BTreeMap<String, String>) -> Result<()> {
        for (name, value) in &self.headers {
            insert(headers, name.clone(), value.clone());
        }
        Ok(())
    }
}

impl fmt::Debug for StaticHeaderAuth {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("StaticHeaderAuth")
            .field("headers", &self.headers.keys().collect::<Vec<_>>())
            .finish_non_exhaustive()
    }
}
