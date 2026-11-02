//! Actual auth-hook dispatch with per-call credential capture.

use std::{collections::BTreeMap, sync::Arc};

use async_trait::async_trait;
use json_http::{DynJsonHttpAuth, JsonHttpAuth, Result};

use crate::ErrorContext;

pub(crate) fn capture(context: ErrorContext, inner: DynJsonHttpAuth) -> DynJsonHttpAuth {
    Arc::new(CapturingAuth { context, inner })
}

struct CapturingAuth {
    context: ErrorContext,
    inner: DynJsonHttpAuth,
}

#[async_trait]
impl JsonHttpAuth for CapturingAuth {
    async fn apply_headers(&self, headers: &mut BTreeMap<String, String>) -> Result<()> {
        let before = headers.clone();
        let result = self.inner.apply_headers(headers).await;
        for (name, value) in headers.iter() {
            if before.get(name) != Some(value) || credential_header(name) {
                self.context.add_secret(value);
                if let Some(token) = value.split_whitespace().last() {
                    self.context.add_secret(token);
                }
            }
        }
        match result {
            Ok(()) => Ok(()),
            Err(_) => Err(json_http::Error::auth("authentication hook failed")),
        }
    }
}

fn credential_header(name: &str) -> bool {
    matches!(
        name.to_ascii_lowercase().as_str(),
        "authorization" | "proxy-authorization" | "x-api-key" | "x-goog-api-key" | "api-key"
    )
}
