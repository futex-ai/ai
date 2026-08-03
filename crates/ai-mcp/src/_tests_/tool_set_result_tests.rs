//! Truncation-envelope invariant tests.

use super::{
    MIN_RESPONSE_BYTES, MIN_SUCCESS_RESPONSE_BYTES, empty_error_truncation_envelope,
    empty_truncation_envelope,
};

#[test]
fn minimum_response_bytes_cover_success_and_error_envelopes() {
    let success = serde_json::to_vec(&empty_truncation_envelope()).unwrap();
    let error = serde_json::to_vec(&empty_error_truncation_envelope()).unwrap();

    assert_eq!(success.len(), MIN_SUCCESS_RESPONSE_BYTES);
    assert_eq!(MIN_SUCCESS_RESPONSE_BYTES, 31);
    assert_eq!(MIN_RESPONSE_BYTES, error.len());
    assert_eq!(MIN_RESPONSE_BYTES, 47);
}
