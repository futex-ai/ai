//! Video generation boundary tests.

use serde_json::json;
use thiserror::Error;

use crate::{
    GeneratedVideo, ModelUsage, VideoGenerationAspect, VideoGenerationDuration,
    VideoGenerationError, VideoGenerationInputImage, VideoGenerationRequest,
    VideoGenerationResolution, VideoGenerationResponse,
};

#[test]
fn request_defaults_and_serde_are_stable() {
    let request: VideoGenerationRequest = serde_json::from_value(json!({
        "prompt": "A bird takes flight"
    }))
    .unwrap();
    assert_eq!(
        request,
        VideoGenerationRequest {
            prompt: "A bird takes flight".to_owned(),
            input_image: None,
            aspect: VideoGenerationAspect::Landscape,
            duration: VideoGenerationDuration::Seconds4,
            resolution: VideoGenerationResolution::P720,
        }
    );

    let request = VideoGenerationRequest {
        prompt: "Animate the frame".to_owned(),
        input_image: Some(VideoGenerationInputImage {
            data: vec![1, 2, 3],
            mime_type: "image/png".to_owned(),
        }),
        aspect: VideoGenerationAspect::Portrait,
        duration: VideoGenerationDuration::Seconds8,
        resolution: VideoGenerationResolution::P720,
    };
    assert_eq!(
        serde_json::to_value(request).unwrap(),
        json!({
            "prompt": "Animate the frame",
            "input_image": {"data": [1, 2, 3], "mime_type": "image/png"},
            "aspect": "portrait",
            "duration": "seconds8",
            "resolution": "p720"
        })
    );
}

#[test]
fn response_serde_and_duration_metadata_are_stable() {
    let response = VideoGenerationResponse {
        provider: "openai".to_owned(),
        model_id: "sora-2".to_owned(),
        video: GeneratedVideo {
            data: vec![0, 1],
            mime_type: "video/mp4".to_owned(),
            duration_seconds: 8,
            width: 720,
            height: 1280,
        },
        usage: ModelUsage::default(),
    };
    let value = serde_json::to_value(&response).unwrap();
    assert_eq!(
        serde_json::from_value::<VideoGenerationResponse>(value).unwrap(),
        response
    );
    assert_eq!(VideoGenerationDuration::Seconds4.seconds(), 4);
    assert_eq!(VideoGenerationDuration::Seconds8.seconds(), 8);
}

#[test]
fn error_helpers_preserve_typed_context() {
    assert!(matches!(
        VideoGenerationError::unsupported_media_type("image/tiff"),
        VideoGenerationError::UnsupportedMediaType { content_type }
            if content_type == "image/tiff"
    ));
    assert!(matches!(
        VideoGenerationError::content_policy("openai", "sora-2", "blocked"),
        VideoGenerationError::ContentPolicy { provider, model_id, message }
            if provider == "openai" && model_id == "sora-2" && message == "blocked"
    ));
    assert!(matches!(
        VideoGenerationError::no_video("google", "veo"),
        VideoGenerationError::NoVideo { .. }
    ));
    assert!(matches!(
        VideoGenerationError::timed_out("google", "veo"),
        VideoGenerationError::TimedOut { .. }
    ));
    assert!(matches!(
        VideoGenerationError::rate_limited("openai", "sora-2", "slow down"),
        VideoGenerationError::RateLimited { .. }
    ));
    assert!(matches!(
        VideoGenerationError::transient_provider("openai", "sora-2", "retry"),
        VideoGenerationError::TransientProvider { .. }
    ));
    assert!(matches!(
        VideoGenerationError::provider("openai", "sora-2", "rejected"),
        VideoGenerationError::Provider { .. }
    ));
}

#[derive(Debug, Error)]
#[error("test source")]
struct TestSourceError;

#[test]
fn internal_error_retains_tracked_definition_location() {
    let expected_line = line!() + 1;
    let error = VideoGenerationError::internal(TestSourceError);
    assert_eq!(
        error.to_string(),
        "[ai_interface/video_generator] internal error"
    );
    let VideoGenerationError::Internal(internal) = error else {
        panic!("expected internal error");
    };

    assert_eq!(
        internal.defined_at().module_path(),
        "ai_interface::video_generator"
    );
    assert_eq!(internal.caller_at().file(), file!());
    assert_eq!(internal.caller_at().line(), expected_line);
    assert_eq!(internal.source_ref().to_string(), "test source");
}
