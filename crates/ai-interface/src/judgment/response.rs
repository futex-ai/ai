//! Normalized judgment response DTO.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::ModelUsage;

use super::JudgmentAnswer;

/// Normalized response from one judgment call.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct JudgmentResponse {
    /// Provider that evaluated the request.
    pub provider: String,
    /// Configured provider model identifier.
    pub model_id: String,
    /// Provider-reported model identifier used for the call.
    pub resolved_model_id: String,
    /// Typed answers keyed by caller-provided question ids.
    pub answers: BTreeMap<String, JudgmentAnswer>,
    /// Provider-reported normalized token usage.
    pub usage: ModelUsage,
}
