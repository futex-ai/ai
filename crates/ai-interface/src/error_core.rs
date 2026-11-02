//! Provider identity and response facts shared by all modality errors.

use std::time::Duration;

use serde::{Deserialize, Serialize};

use crate::ErrorKind;

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
/// Shared failure details without an internal source object.
pub struct ErrorCore {
    /// Precise failure category used by model policies.
    pub kind: ErrorKind,
    /// Stable identity of the adapter that raised the error.
    pub provider: String,
    /// Provider model identifier selected for the call.
    pub model_id: String,
    /// Credential-free diagnostic from the provider or adapter.
    pub message: String,
    /// Wait requested by the failed response's Retry-After header.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub retry_after: Option<Duration>,
    /// Request identifier supplied by the failed response.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub provider_request_id: Option<String>,
}

impl ErrorCore {
    /// Builds failure details before optional response metadata is available.
    pub fn new(
        kind: ErrorKind,
        provider: impl Into<String>,
        model_id: impl Into<String>,
        message: impl Into<String>,
    ) -> Self {
        Self {
            kind,
            provider: provider.into(),
            model_id: model_id.into(),
            message: message.into(),
            retry_after: None,
            provider_request_id: None,
        }
    }

    pub(crate) fn local(kind: ErrorKind, component: &str, message: impl Into<String>) -> Self {
        Self::new(kind, "local", component, message)
    }
}
