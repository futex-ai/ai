//! Assertions read original Gemini parts through the public provider state.

use ai_interface::ModelResponse;
use serde_json::Value;

pub(super) fn original_parts(response: &ModelResponse) -> Value {
    Value::Array(
        response
            .parts
            .iter()
            .filter_map(|part| part.provider_state())
            .flat_map(|state| state.payload["parts"].as_array().unwrap().iter().cloned())
            .collect(),
    )
}
