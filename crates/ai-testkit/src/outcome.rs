//! Serializable completion results with a stable, secret-free error surface.

use ai_interface::{ErrorCore, ModelAttempt, ModelError, ModelResponse};
use json_http::test_support::Redactor;
use serde::{Deserialize, Serialize};

use crate::event_snapshot::ModelEventSnapshot;

/// One scenario's ordered step results.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct ScenarioOutcome {
    /// Completed steps followed by the first failure, if any.
    pub steps: Vec<StepOutcome>,
    /// Exchanges left when an adapter rejected a request before transport.
    pub remaining_exchanges: usize,
}

/// A model call's response or its error fields.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum StepOutcome {
    /// Successful completion.
    Response {
        /// Ordered events from this model call.
        #[serde(default)]
        events: Vec<ModelEventSnapshot>,
        /// Full normalized model result, including provider context and usage.
        response: Box<ModelResponse>,
    },
    /// Failed completion.
    Error {
        /// Ordered events received before this model call failed.
        #[serde(default)]
        events: Vec<ModelEventSnapshot>,
        /// Shared model error core and its earlier sanitized attempts.
        error: ErrorOutcome,
    },
    /// The tool loop's first response contained no tool call.
    MissingToolCall,
}

/// One sanitized shared error core and its earlier failed attempts.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct ErrorOutcome {
    /// Shared failure details with optional response metadata.
    #[serde(flatten)]
    pub core: ErrorCore,
    /// Earlier failed attempts in chronological order.
    pub attempts: Vec<ModelAttempt>,
}

impl ErrorOutcome {
    pub(crate) fn capture(mut error: ModelError, redactor: &Redactor) -> Self {
        let mut core = error.core().clone();
        core.provider = redactor.text(&core.provider);
        core.model_id = redactor.text(&core.model_id);
        core.message = redactor.text(&core.message);
        core.provider_request_id = core
            .provider_request_id
            .as_ref()
            .map(|id| redactor.text(id));
        let attempts = error
            .take_attempts()
            .into_iter()
            .map(|mut attempt| {
                attempt.provider = redactor.text(&attempt.provider);
                attempt.model_id = redactor.text(&attempt.model_id);
                attempt.message = redactor.text(&attempt.message);
                attempt
            })
            .collect();
        Self { core, attempts }
    }
}
