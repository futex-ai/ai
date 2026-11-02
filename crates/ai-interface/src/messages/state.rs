//! Opaque replay state owned and decoded by one provider adapter.

use serde::{Deserialize, Serialize, de::DeserializeOwned};
use serde_json::{Map, Value};

use crate::{ModelError, ModelResult, ProviderId};

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
/// Replay data attached to the part that produced it.
pub struct ProviderState {
    /// Identity of the adapter that owns the payload.
    pub provider: ProviderId,
    /// Model that produced the state, or empty for converted state.
    pub model: String,
    /// Positive version of the provider-owned payload format.
    pub version: u32,
    /// Provider-owned JSON object, opaque to every other component.
    pub payload: Map<String, Value>,
}

impl ProviderState {
    /// Decodes own state and ignores the state of another provider.
    pub fn decode<T: DeserializeOwned>(
        &self,
        provider: &ProviderId,
        model_id: &str,
        version: u32,
    ) -> ModelResult<Option<T>> {
        if self.provider != *provider {
            return Ok(None);
        }
        if self.version != version || self.version == 0 {
            return Err(ModelError::provider(
                provider.as_str(),
                model_id,
                "unsupported provider state version",
            ));
        }
        match serde_json::from_value(Value::Object(self.payload.clone())) {
            Ok(payload) => Ok(Some(payload)),
            Err(_) => Err(ModelError::provider(
                provider.as_str(),
                model_id,
                "invalid provider state payload",
            )),
        }
    }
}
