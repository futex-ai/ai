//! OpenAI video job response tests.

use crate::openai::stream_support::error_context;

use serde_json::json;

use super::super::response::{JobState, parse_job};

#[test]
fn parses_pending_and_completed_jobs() {
    assert_eq!(
        parse_job(
            &error_context("sora-2"),
            json!({"id": "video_1", "status": "queued"})
        )
        .unwrap(),
        ("video_1".to_owned(), JobState::Pending)
    );
    assert_eq!(
        parse_job(
            &error_context("sora-2"),
            json!({"id": "video_1", "status": "completed"})
        )
        .unwrap(),
        ("video_1".to_owned(), JobState::Completed)
    );
}

#[test]
fn failed_policy_job_is_typed_separately() {
    let error = parse_job(
        &error_context("sora-2"),
        json!({
            "id": "video_1",
            "status": "failed",
            "error": {"code": "content_policy_violation", "message": "blocked"}
        }),
    )
    .unwrap_err();
    assert!(
        matches!(error, ref error if error.core().kind == ai_interface::ErrorKind::ContentPolicy && (error.core().message == "blocked"))
    );
    let rejected = parse_job(
        &error_context("sora-2"),
        json!({"id":"video_1","status":"failed","error":{"code":"invalid_prompt","message":"invalid prompt"}}),
    ).unwrap_err();
    assert_eq!(
        rejected.core().kind,
        ai_interface::ErrorKind::InvalidRequest
    );
    assert_eq!(rejected.core().message, "invalid prompt");
}

#[test]
fn rejects_invalid_ids_unknown_states_and_malformed_jobs() {
    for body in [
        json!({"id": "../../secret", "status": "queued"}),
        json!({"id": "video_1", "status": "mystery"}),
        json!({"status": "queued"}),
    ] {
        assert!(
            matches!(parse_job(&error_context("sora-2"), body), Err(ref error) if error.core().kind == ai_interface::ErrorKind::InvalidOutput)
        );
    }
}
