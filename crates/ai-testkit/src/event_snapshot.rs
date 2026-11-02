//! Typed, serializable snapshots of public model events.

use ai_interface::{
    Effort, FinishReason, ModelAttemptError, ModelEvent, ModelUsage, ProviderState,
};
use json_http::test_support::Redactor;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::error::{ErrorResultExt, Result};

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[non_exhaustive]
/// Sanitized snapshot of one ordered event from a model attempt or fallback.
pub enum ModelEventSnapshot {
    /// An append-only fragment of assistant text.
    TextDelta {
        /// Zero-based position of this text part.
        part_index: usize,
        /// Exact nonempty text to append to the part.
        delta: String,
        /// Opaque replay state of this part.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        provider_state: Option<ProviderState>,
    },
    /// An append-only fragment of provider-exposed reasoning.
    ReasoningDelta {
        /// Zero-based position of this reasoning part.
        part_index: usize,
        /// Exact nonempty reasoning text to append to the part.
        delta: String,
        /// Opaque replay state of this part.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        provider_state: Option<ProviderState>,
    },
    /// The start of one tool call.
    ToolCallStart {
        /// Zero-based position of this tool-call part.
        part_index: usize,
        /// Provider or adapter tool-call identifier.
        id: String,
        /// Tool name requested by the model.
        name: String,
        /// Optional runtime operation identifier retained for execution.
        #[serde(skip_serializing_if = "Option::is_none")]
        operation_id: Option<String>,
    },
    /// An append-only fragment of tool-call argument JSON.
    ToolCallArgumentsDelta {
        /// Zero-based position of the started tool-call part.
        part_index: usize,
        /// Exact nonempty argument fragment.
        delta: String,
    },
    /// The end of a tool call's argument text.
    ToolCallEnd {
        /// Zero-based position of the started tool-call part.
        part_index: usize,
        /// Opaque replay state of this call.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        provider_state: Option<ProviderState>,
    },
    /// The final usage of one successful attempt.
    Usage {
        /// Token counts and prices for this attempt.
        usage: ModelUsage,
    },
    /// The terminal metadata of one successful attempt.
    Finish {
        /// Normalized reason the attempt stopped.
        finish_reason: FinishReason,
        /// Provider that served the attempt.
        provider: String,
        /// Concrete provider model identifier.
        model_id: String,
        /// Text id of the selected model spec, when available.
        #[serde(skip_serializing_if = "Option::is_none")]
        catalog_model_id: Option<String>,
        /// Resolved portable effort, when available.
        #[serde(rename = "effort", skip_serializing_if = "Option::is_none")]
        thinking_level: Option<Effort>,
        /// Validated structured output, when requested and completed.
        #[serde(skip_serializing_if = "Option::is_none")]
        structured_output: Option<Value>,
    },
    /// A failed attempt whose events the consumer must discard.
    Fallback {
        /// Serializable failure details of the discarded attempt.
        error: ModelAttemptError,
    },
}

impl ModelEventSnapshot {
    /// Captures a sanitized copy while the original event remains available to the fold.
    pub fn capture(event: &ModelEvent, redactor: &Redactor) -> Result<Self> {
        let value = serde_json::to_value(event).context("serialize model event")?;
        serde_json::from_value(redactor.value(&value)).context("sanitize model event")
    }
}
