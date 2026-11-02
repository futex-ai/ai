//! Typed provider status envelopes for compatible streams.

use super::{super::dialect::Dialect, types::BaseResponseEnvelope};
use ai_interface::ModelError;
use serde_json::Value;

pub(crate) fn stream_base_response_error(
    dialect: &Dialect,
    model_id: &str,
    body: &Value,
) -> Result<Option<ModelError>, serde_json::Error> {
    let envelope: BaseResponseEnvelope = serde_json::from_value(body.clone())?;
    Ok(dialect
        .base_error
        .and_then(|classify| classify(model_id, envelope.base_resp.as_ref())))
}
