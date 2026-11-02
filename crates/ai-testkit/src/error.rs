//! Typed failures of recording storage and conformance setup.

use internal_error::InternalError;
use thiserror::Error;

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
    /// Storage, serialization, or adapter construction failed.
    #[error("[ai_testkit/error] internal error: {0}")]
    Internal(#[from] InternalError),
}

/// Result of test kit setup or storage.
pub type Result<T> = std::result::Result<T, Error>;
