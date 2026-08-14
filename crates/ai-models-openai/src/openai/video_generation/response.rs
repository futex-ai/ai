//! OpenAI video job response parsing.

use ai_interface::{VideoGenerationError, VideoGenerationResult};
use serde::Deserialize;
use serde_json::Value;

const PROVIDER: &str = "openai";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum JobState {
    Pending,
    Completed,
}

#[derive(Debug, Deserialize)]
struct VideoJob {
    id: String,
    status: String,
    error: Option<VideoJobError>,
}

#[derive(Debug, Deserialize)]
struct VideoJobError {
    code: Option<String>,
    message: Option<String>,
}

pub(super) fn parse_job(model_id: &str, body: Value) -> VideoGenerationResult<(String, JobState)> {
    let job: VideoJob = match serde_json::from_value(body) {
        Ok(job) => job,
        Err(source) => {
            return Err(VideoGenerationError::provider(
                PROVIDER,
                model_id,
                source.to_string(),
            ));
        }
    };
    if !valid_job_id(&job.id) {
        return Err(VideoGenerationError::provider(
            PROVIDER,
            model_id,
            "provider returned an invalid video job id",
        ));
    }
    match job.status.as_str() {
        "queued" | "in_progress" => Ok((job.id, JobState::Pending)),
        "completed" => Ok((job.id, JobState::Completed)),
        "failed" => Err(failed_job(model_id, job.error)),
        _ => Err(VideoGenerationError::provider(
            PROVIDER,
            model_id,
            "provider returned an unknown video job status",
        )),
    }
}

fn failed_job(model_id: &str, error: Option<VideoJobError>) -> VideoGenerationError {
    let is_content_policy = error
        .as_ref()
        .and_then(|error| error.code.as_deref())
        .is_some_and(is_content_policy_code);
    let message = error
        .and_then(|error| error.message)
        .unwrap_or_else(|| "video generation failed".to_owned());
    if is_content_policy {
        return VideoGenerationError::content_policy(PROVIDER, model_id, message);
    }
    VideoGenerationError::provider(PROVIDER, model_id, message)
}

fn is_content_policy_code(code: &str) -> bool {
    matches!(
        code,
        "content_policy_violation"
            | "moderation_blocked"
            | "safety_violation"
            | "video_generation_safety_violation"
    )
}

fn valid_job_id(id: &str) -> bool {
    !id.is_empty()
        && id.len() <= 512
        && id
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_'))
}
