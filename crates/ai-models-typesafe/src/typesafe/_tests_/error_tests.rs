//! TypeSafe request error classification tests.

use ai_interface::JudgmentError;

use super::{AUTH_HOOK_FAILED, classify_request_error};

const API_KEY: &str = "typesafe-secret-key";

#[test]
fn auth_errors_use_the_fixed_non_secret_diagnostic() {
    let error = classify_request_error(
        json_http::Error::auth("Authorization: Bearer typesafe-secret-key"),
        "jev-latest",
        &[],
    );

    let JudgmentError::TransientProvider {
        provider,
        model_id,
        message,
    } = &error
    else {
        panic!("expected a transient provider error, got {error}");
    };
    assert_eq!(provider, "typesafe");
    assert_eq!(model_id, "jev-latest");
    assert_eq!(message, AUTH_HOOK_FAILED);
    assert!(!error.to_string().contains(API_KEY));
    assert!(!format!("{error:?}").contains(API_KEY));
}
