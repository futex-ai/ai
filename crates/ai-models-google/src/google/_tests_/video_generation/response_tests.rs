//! Google video operation response tests.

use ai_interface::VideoGenerationError;
use serde_json::json;

use super::super::response::{OperationState, parse_operation};

const OPERATION: &str = "models/veo-3.1-generate-preview/operations/op_1";

#[test]
fn parses_pending_and_completed_operations() {
    assert_eq!(
        parse_operation("veo", json!({"name": OPERATION})).unwrap(),
        (OPERATION.to_owned(), OperationState::Pending)
    );
    assert_eq!(
        parse_operation(
            "veo",
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
        "veo",
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
    assert!(matches!(
        policy,
        VideoGenerationError::ContentPolicy { message, .. } if message == "blocked likeness"
    ));

    let transient = parse_operation(
        "veo",
        json!({
            "name": OPERATION,
            "done": true,
            "error": {"code": 14, "status": "UNAVAILABLE", "message": "retry"}
        }),
    )
    .unwrap_err();
    assert!(matches!(
        transient,
        VideoGenerationError::TransientProvider { .. }
    ));
}

#[test]
fn rejects_invalid_names_and_missing_videos() {
    assert!(matches!(
        parse_operation("veo", json!({"name": "../operation"})),
        Err(VideoGenerationError::Provider { .. })
    ));
    assert!(matches!(
        parse_operation(
            "veo",
            json!({"name": OPERATION, "done": true, "response": {}})
        ),
        Err(VideoGenerationError::NoVideo { .. })
    ));
}
