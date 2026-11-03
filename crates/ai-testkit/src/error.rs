//! Typed failures of recording storage and conformance setup.

use internal_error::InternalError;
use thiserror::Error;

use crate::{provider::Provider, scenario::Scenario};

/// Errors outside the provider completion result being measured.
#[derive(Debug, Error, internal_error::ErrorContract)]
pub enum Error {
    /// The CLI provider name is unknown.
    #[error("[ai_testkit/provider] unknown provider `{name}`")]
    UnknownProvider {
        /// Supplied provider name.
        name: String,
    },
    /// A recording names an entry absent from the current provider catalog.
    #[error("[ai_testkit/provider] catalog model `{id}` is unavailable")]
    UnknownModel {
        /// Stored catalog model id.
        id: String,
    },
    /// A tool scenario cannot continue without its first tool call.
    #[error("[ai_testkit/scenario] the first step did not return a tool call")]
    MissingToolCall,
    /// An outbound request differs from its sanitized recorded exchange.
    #[error(
        "[ai_testkit/runner] {provider_name}/{scenario_name} exchange index {exchange_index} request mismatch\nexpected:\n{expected}\nactual:\n{actual}",
        provider_name = .provider.name(),
        scenario_name = .scenario.name()
    )]
    RequestMismatch {
        /// Provider whose adapter sent the request.
        provider: Provider,
        /// Scenario being replayed.
        scenario: Scenario,
        /// Zero-based exchange position.
        exchange_index: usize,
        /// Pretty JSON of the recorded request, or null when absent.
        expected: String,
        /// Pretty JSON of the captured request, or null when absent.
        actual: String,
    },
    /// Storage, serialization, or adapter construction failed.
    #[error("[ai_testkit/error] internal error: {0}")]
    Internal(#[from] InternalError),
}

/// Result of test kit setup or storage.
pub type Result<T> = std::result::Result<T, Error>;
