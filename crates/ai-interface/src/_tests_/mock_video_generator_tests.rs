//! Built-in mock video generator tests.

use crate::{
    GeneratedVideo, MockVideoGenerator, VideoGenerationAspect, VideoGenerationDuration,
    VideoGenerationError, VideoGenerationRequest, VideoGenerator,
};

#[tokio::test]
async fn default_mock_returns_a_valid_deterministic_mp4() {
    let response = MockVideoGenerator::default()
        .generate(&VideoGenerationRequest {
            prompt: "A blue circle spins".to_owned(),
            aspect: VideoGenerationAspect::Portrait,
            duration: VideoGenerationDuration::Seconds8,
            ..VideoGenerationRequest::default()
        })
        .await
        .unwrap();

    assert_eq!(response.provider, "mock");
    assert_eq!(response.model_id, "mock-video");
    assert_eq!(response.video.mime_type, "video/mp4");
    assert_eq!(&response.video.data[4..8], b"ftyp");
    assert_eq!(response.video.duration_seconds, 8);
    assert_eq!((response.video.width, response.video.height), (720, 1280));
}

#[tokio::test]
async fn configurable_mock_returns_supplied_video() {
    let video = GeneratedVideo {
        data: vec![1, 2, 3],
        mime_type: "video/mp4".to_owned(),
        duration_seconds: 4,
        width: 1,
        height: 1,
    };
    let response = MockVideoGenerator::new(video.clone())
        .generate(&VideoGenerationRequest {
            prompt: "Go".to_owned(),
            ..VideoGenerationRequest::default()
        })
        .await
        .unwrap();
    assert_eq!(response.video.data, video.data);
}

#[tokio::test]
async fn mock_rejects_a_blank_prompt() {
    let error = MockVideoGenerator::default()
        .generate(&VideoGenerationRequest::default())
        .await
        .unwrap_err();
    assert!(matches!(error, VideoGenerationError::EmptyPrompt));
}
