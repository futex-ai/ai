//! Truncation-envelope invariant tests.

use super::{MIN_RESPONSE_BYTES, empty_truncation_envelope};

#[test]
fn minimum_response_bytes_matches_the_empty_envelope() {
    let serialized = serde_json::to_vec(&empty_truncation_envelope()).unwrap();

    assert_eq!(MIN_RESPONSE_BYTES, serialized.len());
}
