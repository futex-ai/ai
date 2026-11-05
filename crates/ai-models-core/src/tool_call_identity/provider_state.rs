//! Stable hashing of opaque provider replay state.

use ai_interface::ProviderState;
use serde_json::Value;

use super::hasher::StableHasher;

impl StableHasher {
    pub(super) fn write_provider_state(&mut self, state: Option<&ProviderState>) {
        let Some(state) = state else {
            self.write_bool(false);
            return;
        };
        self.write_bool(true);
        self.write_str(state.provider.as_str());
        self.write_str(&state.model);
        self.write_u64(u64::from(state.version));
        self.write_json(&Value::Object(state.payload.clone()));
    }
}
