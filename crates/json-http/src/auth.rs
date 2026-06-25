//! Auth hook types for JSON HTTP requests.

use std::{collections::BTreeMap, sync::Arc};

use async_trait::async_trait;

use crate::Result;

/// Shared dynamic auth hook alias.
pub type DynJsonHttpAuth = Arc<dyn JsonHttpAuth>;

#[async_trait]
/// Applies request headers before a JSON HTTP request is dispatched.
pub trait JsonHttpAuth: Send + Sync {
    /// Mutates the outgoing request headers in place.
    async fn apply_headers(&self, headers: &mut BTreeMap<String, String>) -> Result<()>;
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
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
        headers.extend(self.headers.clone());
        Ok(())
    }
}
