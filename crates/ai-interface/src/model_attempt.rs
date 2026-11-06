//! Serializable records of failed fallback attempts.

use serde::{Deserialize, Serialize, Serializer};

use crate::{ErrorKind, ModelError};

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
/// One failed model attempt, without an internal source object.
pub struct ModelAttempt {
    /// Stable provider identity of the failed attempt.
    pub provider: String,
    /// Requested provider model identifier.
    pub model_id: String,
    /// Precise failure category.
    pub kind: ErrorKind,
    /// Credential-free failure diagnostic.
    pub message: String,
    /// Whether this attempt returned at least one model event.
    pub emitted_events: bool,
}

pub(crate) fn serialize_model_error<S>(error: &ModelError, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    error.core().serialize(serializer)
}
