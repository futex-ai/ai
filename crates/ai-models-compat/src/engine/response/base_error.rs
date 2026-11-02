//! Typed provider status envelopes for compatible streams.

use super::{super::dialect::Dialect, types::BaseResponseEnvelope};
use ai_interface::ModelError;
use ai_models_core::ErrorContext;
use serde_json::Value;

pub(crate) fn stream_base_response_error(
    dialect: &Dialect,
    context: &ErrorContext,
    body: &Value,
) -> Result<Option<ModelError>, serde_json::Error> {
    let envelope: BaseResponseEnvelope = serde_json::from_value(body.clone())?;
    Ok(dialect
        .base_error
        .and_then(|classify| classify(context, envelope.base_resp.as_ref())))
}
