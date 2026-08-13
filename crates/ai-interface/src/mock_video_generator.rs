//! Built-in mock video generator for development and tests.

use async_trait::async_trait;

use crate::{
    GeneratedVideo, ModelUsage, VideoGenerationAspect, VideoGenerationError,
    VideoGenerationRequest, VideoGenerationResponse, VideoGenerationResult, VideoGenerator,
};

const DEFAULT_MP4: &[u8] = &[
    0, 0, 0, 24, b'f', b't', b'y', b'p', b'i', b's', b'o', b'm', 0, 0, 0, 0, b'i', b's', b'o',
    b'm', b'm', b'p', b'4', b'2',
];

/// Deterministic mock video generator used by development and tests.
#[derive(Clone, Debug)]
pub struct MockVideoGenerator {
    video: GeneratedVideo,
}

impl MockVideoGenerator {
    /// Builds a mock generator that returns the supplied video.
    pub fn new(video: GeneratedVideo) -> Self {
        Self { video }
    }
}

impl Default for MockVideoGenerator {
    fn default() -> Self {
        Self::new(GeneratedVideo {
            data: DEFAULT_MP4.to_vec(),
            mime_type: "video/mp4".to_owned(),
            duration_seconds: 4,
            width: 1280,
            height: 720,
        })
    }
}

#[async_trait]
impl VideoGenerator for MockVideoGenerator {
    async fn generate(
        &self,
        request: &VideoGenerationRequest,
    ) -> VideoGenerationResult<VideoGenerationResponse> {
        if request.prompt.trim().is_empty() {
            return Err(VideoGenerationError::EmptyPrompt);
        }
        let mut video = self.video.clone();
        video.duration_seconds = request.duration.seconds();
        (video.width, video.height) = match request.aspect {
            VideoGenerationAspect::Landscape => (1280, 720),
            VideoGenerationAspect::Portrait => (720, 1280),
        };
        Ok(VideoGenerationResponse {
            provider: "mock".to_owned(),
            model_id: "mock-video".to_owned(),
            video,
            usage: ModelUsage::default(),
        })
    }
}
