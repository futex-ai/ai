//! OpenAI video job response tests.

use ai_interface::VideoGenerationError;
use serde_json::json;

use super::super::response::{JobState, parse_job};

#[test]
fn parses_pending_and_completed_jobs() {
    assert_eq!(
        parse_job("sora-2", json!({"id": "video_1", "status": "queued"})).unwrap(),
        ("video_1".to_owned(), JobState::Pending)
    );
    assert_eq!(
        parse_job("sora-2", json!({"id": "video_1", "status": "completed"})).unwrap(),
        ("video_1".to_owned(), JobState::Completed)
    );
}

#[test]
fn failed_policy_job_is_typed_separately() {
    let error = parse_job(
        "sora-2",
        json!({
            "id": "video_1",
            "status": "failed",
            "error": {"code": "content_policy_violation", "message": "blocked"}
        }),
    )
    .unwrap_err();
    assert!(matches!(
        error,
        VideoGenerationError::ContentPolicy { message, .. } if message == "blocked"
    ));
}

#[test]
fn rejects_invalid_ids_unknown_states_and_malformed_jobs() {
    for body in [
        json!({"id": "../../secret", "status": "queued"}),
        json!({"id": "video_1", "status": "mystery"}),
        json!({"status": "queued"}),
    ] {
        assert!(matches!(
            parse_job("sora-2", body),
            Err(VideoGenerationError::Provider { .. })
        ));
    }
}
