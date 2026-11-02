//! Fallback snapshots keep one sanitized shared core and explicit event visibility.

use std::time::Duration;

use ai_interface::{ErrorCore, ErrorKind, ModelError, ModelEvent};
use json_http::test_support::Redactor;
use serde_json::json;

use crate::event_snapshot::ModelEventSnapshot;

#[test]
fn fallback_snapshot_keeps_the_core_metadata_and_emitted_event_flag() {
    let mut core = ErrorCore::new(
        ErrorKind::Interrupted,
        "provider-secret-token",
        "model-secret-token",
        "closed secret-token",
    );
    core.retry_after = Some(Duration::from_millis(1500));
    core.provider_request_id = Some("request-secret-token".to_owned());
    let event = ModelEvent::Fallback {
        error: ModelError::new(core),
        emitted_events: false,
    };
    let snapshot =
        ModelEventSnapshot::capture(&event, &Redactor::new(["secret-token".to_owned()])).unwrap();

    assert_eq!(
        serde_json::to_value(snapshot).unwrap(),
        json!({"Fallback": {
            "error": {
                "kind": "Interrupted", "provider": "provider-[redacted]", "model_id": "model-[redacted]",
                "message": "closed [redacted]", "retry_after": {"secs": 1, "nanos": 500000000},
                "provider_request_id": "request-[redacted]"
            },
            "emitted_events": false
        }})
    );
}
