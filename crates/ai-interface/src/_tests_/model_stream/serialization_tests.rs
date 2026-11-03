//! Serializable events and failed attempts without cloneable source errors.

use std::{error::Error, fmt};

use serde_json::json;

use crate::{
    ErrorKind, FinishReason, ModelAttempt, ModelControl, ModelError, ModelEvent, ModelResponse,
    SafeErrorSource,
};

use super::fixtures::{finish, fold, usage};

#[derive(Debug)]
struct Source;

impl fmt::Display for Source {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("source failure")
    }
}

impl Error for Source {}

#[test]
fn fallback_serialization_preserves_error_fields_without_cloning_the_source() {
    let cases = [
        (
            ModelError::rate_limited("p", "m", "limited"),
            json!({"kind": "RateLimited", "provider": "p", "model_id": "m", "message": "limited"}),
        ),
        (
            ModelError::transient("p", "m", "unavailable"),
            json!({"kind": "Transient", "provider": "p", "model_id": "m", "message": "unavailable"}),
        ),
        (
            ModelError::interrupted("p", "m", "closed"),
            json!({"kind": "Interrupted", "provider": "p", "model_id": "m", "message": "closed"}),
        ),
        (
            ModelError::invalid_request("p", "m", "rejected"),
            json!({"kind": "InvalidRequest", "provider": "p", "model_id": "m", "message": "rejected"}),
        ),
        (
            ModelError::context_limit_exceeded("p", "m", "large"),
            json!({"kind": "ContextLimitExceeded", "provider": "p", "model_id": "m", "message": "large"}),
        ),
        (
            ModelError::unsupported_control("p", "m", ModelControl::Effort),
            json!({"kind": {"UnsupportedControl": "Effort"}, "provider": "p", "model_id": "m", "message": "unsupported Effort control"}),
        ),
        (
            ModelError::internal("local", "test", SafeErrorSource::redacted(Source)),
            json!({"kind": "Internal", "provider": "local", "model_id": "test", "message": "internal error"}),
        ),
    ];
    for (error, expected) in cases {
        let event = ModelEvent::Fallback {
            error,
            emitted_events: false,
        };
        assert_eq!(
            serde_json::to_value(&event).unwrap(),
            json!({"Fallback": {"error": expected, "emitted_events": false}})
        );
    }
}

#[test]
fn response_attempt_records_round_trip_and_empty_attempts_keep_the_old_shape() {
    let response = fold(vec![usage(), finish(FinishReason::Stop)]).unwrap();
    let old_shape = serde_json::to_value(&response).unwrap();
    assert_eq!(old_shape.get("attempts"), None);
    assert_eq!(
        serde_json::from_value::<ModelResponse>(old_shape).unwrap(),
        response
    );

    let attempt =
        ModelError::internal("local", "test", SafeErrorSource::redacted(Source)).attempt(false);
    assert_eq!(
        attempt,
        ModelAttempt {
            provider: "local".to_owned(),
            model_id: "test".to_owned(),
            kind: ErrorKind::Internal,
            message: "internal error".to_owned(),
            emitted_events: false,
        }
    );
    assert_eq!(
        serde_json::from_value::<ModelAttempt>(serde_json::to_value(&attempt).unwrap()).unwrap(),
        attempt
    );
}
