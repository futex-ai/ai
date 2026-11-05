//! Serializable events and failed attempts without cloneable source errors.

use std::{error::Error, fmt};

use serde_json::json;

use crate::{
    FinishReason, ModelAttempt, ModelAttemptError, ModelControl, ModelError, ModelEvent,
    ModelResponse,
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
            json!({"RateLimited": {"provider": "p", "model_id": "m", "message": "limited"}}),
        ),
        (
            ModelError::transient_provider("p", "m", "unavailable"),
            json!({"TransientProvider": {"provider": "p", "model_id": "m", "message": "unavailable"}}),
        ),
        (
            ModelError::interrupted("p", "m", "closed"),
            json!({"Interrupted": {"provider": "p", "model_id": "m", "message": "closed"}}),
        ),
        (
            ModelError::provider("p", "m", "rejected"),
            json!({"Provider": {"provider": "p", "model_id": "m", "message": "rejected"}}),
        ),
        (
            ModelError::context_limit_exceeded("p", "m", "large"),
            json!({"ContextLimitExceeded": {"provider": "p", "model_id": "m", "message": "large"}}),
        ),
        (
            ModelError::unsupported_control("p", "m", ModelControl::Effort),
            json!({"UnsupportedControl": {"provider": "p", "model_id": "m", "control": "Effort"}}),
        ),
        (
            ModelError::internal(Source),
            json!({"Internal": {"message": "source failure"}}),
        ),
    ];
    for (error, expected) in cases {
        let event = ModelEvent::Fallback { error };
        assert_eq!(
            serde_json::to_value(&event).unwrap(),
            json!({"Fallback": {"error": expected}})
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

    let attempt = ModelAttempt::from(&ModelError::internal(Source));
    assert_eq!(
        attempt.error,
        ModelAttemptError::Internal {
            message: "source failure".to_owned()
        }
    );
    assert_eq!(
        serde_json::from_value::<ModelAttempt>(serde_json::to_value(&attempt).unwrap()).unwrap(),
        attempt
    );
}
