//! TypeSafe request error classification tests.

use ai_interface::ErrorKind;

use ai_interface::JudgmentError;

use super::{AUTH_HOOK_FAILED, classify_request_error};
use ai_models_core::{ErrorContext, SystemClock};
use std::sync::Arc;

const API_KEY: &str = "typesafe-secret-key";

#[test]
fn auth_errors_use_the_fixed_non_secret_diagnostic() {
    let context = ErrorContext::new("typesafe", "jev-latest", &[], Arc::new(SystemClock));
    let error = classify_request_error(
        json_http::Error::auth("Authorization: Bearer typesafe-secret-key"),
        &context,
        &[],
    );

    let core = error.core();
    let (provider, model_id, message) = (&core.provider, &core.model_id, &core.message);
    assert_eq!(core.kind, ErrorKind::Transient);
    assert_eq!(provider, "typesafe");
    assert_eq!(model_id, "jev-latest");
    assert_eq!(message, AUTH_HOOK_FAILED);
    assert!(!error.to_string().contains(API_KEY));
    assert!(!format!("{error:?}").contains(API_KEY));
    assert!(matches!(error, JudgmentError::Failure { .. }));
}
