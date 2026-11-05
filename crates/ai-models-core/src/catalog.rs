//! Compatibility helper for the contract-owned development model catalog.

use ai_interface::{MOCK_CATALOG, ModelSpec};

/// Returns the model facts used by local development and tests.
pub fn known_mock_models() -> Vec<ModelSpec> {
    MOCK_CATALOG.to_vec()
}
