//! Shared failure categories used by every model modality.

use serde::{Deserialize, Serialize};

use crate::ModelControl;

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
/// The cause of a failed model or modality call.
pub enum ErrorKind {
    /// The request breaks the input contract.
    InvalidRequest,
    /// The provider rejects the key or account.
    RejectedKey,
    /// The provider does not recognize the model.
    UnknownModel,
    /// The model does not accept the input part or media type.
    UnsupportedInput,
    /// The provider output breaks the response contract.
    InvalidOutput,
    /// An image or video provider refuses content for policy reasons.
    ContentPolicy,
    /// The provider asks the caller to reduce its request rate.
    RateLimited,
    /// A temporary provider or transport failure occurs before progress.
    Transient,
    /// A provider stream fails after progress.
    Interrupted,
    /// The input exceeds the model context window.
    ContextLimitExceeded,
    /// The model cannot honor the named explicit control.
    UnsupportedControl(ModelControl),
    /// A video generation job exceeds its total timeout.
    TimedOut,
    /// A local invariant, serialization, or transport capability fails.
    Internal,
}

impl ErrorKind {
    /// Returns whether another model can succeed for this failure category.
    pub fn can_succeed_with_another_model(self) -> bool {
        !matches!(
            self,
            Self::InvalidRequest | Self::ContentPolicy | Self::Internal
        )
    }
}
