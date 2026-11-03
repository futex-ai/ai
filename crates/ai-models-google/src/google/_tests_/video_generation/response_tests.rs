//! Google video operation response tests.

use ai_interface::VideoGenerationError;
use serde_json::json;

use crate::google::google_tests::stream_support::error_context;

use super::super::response::{OperationState, parse_operation};

const OPERATION: &str = "models/custom-video-model/operations/op_1";

#[test]
fn parses_pending_and_completed_operations() {
    assert_eq!(parse_operation(&error_context("veo"), &Default::default(), json!({"name":OPERATION,"error":{"code":16,"status":"UNAUTHENTICATED","message":"pending diagnostic"}})).map_err(|error|error.core().kind), Ok((OPERATION.to_owned(),OperationState::Pending)));
    assert_eq!(
        parse_operation(
            &error_context("veo"),
            &Default::default(),
            json!({"name": OPERATION})
        )
        .unwrap(),
        (OPERATION.to_owned(), OperationState::Pending)
    );
    assert_eq!(
        parse_operation(
            &error_context("veo"),
            &Default::default(),
            json!({
                "name": OPERATION,
                "done": true,
                "response": {"generateVideoResponse": {"generatedSamples": [
                    {"video": {"uri": "https://generativelanguage.googleapis.com/v1beta/files/1"}}
                ]}}
            })
        )
        .unwrap(),
        (
            OPERATION.to_owned(),
            OperationState::Completed {
                download_uri: "https://generativelanguage.googleapis.com/v1beta/files/1".to_owned()
            }
        )
    );
}

#[test]
fn policy_filter_and_operation_errors_are_typed() {
    let policy = parse_operation(
        &error_context("veo"),
        &Default::default(),
        json!({
            "name": OPERATION,
            "done": true,
            "response": {"generateVideoResponse": {
                "raiMediaFilteredCount": 1,
                "raiMediaFilteredReasons": ["blocked likeness"]
            }}
        }),
    )
    .unwrap_err();
    assert!(
        matches!(policy, ref error if error.core().kind == ai_interface::ErrorKind::ContentPolicy && (error.core().message == "blocked likeness"))
    );

    let transient = parse_operation(
        &error_context("veo"),
        &Default::default(),
        json!({
            "name": OPERATION,
            "done": true,
            "error": {"code": 14, "status": "UNAVAILABLE", "message": "retry"}
        }),
    )
    .unwrap_err();
    assert!(
        matches!(transient, ref error if error.core().kind == ai_interface::ErrorKind::Transient)
    );
}

#[test]
fn rejects_invalid_names_and_missing_videos() {
    assert!(
        matches!(parse_operation(&error_context("veo"), &Default::default(), json!({"name": "../operation"})), Err(ref error) if error.core().kind == ai_interface::ErrorKind::InvalidOutput)
    );
    assert!(matches!(
        parse_operation(
            &error_context("veo"),
            &Default::default(),
            json!({"name": OPERATION, "done": true, "response": {}})
        ),
        Err(VideoGenerationError::NoVideo { ref core }) if core.kind == ai_interface::ErrorKind::InvalidOutput
    ));
}
