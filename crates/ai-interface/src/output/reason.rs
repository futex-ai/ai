//! Reasons a tool output remainder cannot be fetched.

use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
/// Reason that remaining bytes cannot be read through `tool_output_read`.
pub enum ToolOutputRemainderUnavailableReason {
    /// The serialized output exceeded the per-output storage limit.
    OutputTooLarge,
    /// The runtime output store exhausted its aggregate storage budget.
    BudgetExhausted,
    /// The output store failed while writing the serialized output.
    StoreUnavailable,
}
