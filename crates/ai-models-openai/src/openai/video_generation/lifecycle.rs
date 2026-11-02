//! One video job retains response facts across parsing, polling, and download.

use ai_interface::{
    ErrorKind, GeneratedVideo, ModelUsage, VideoGenerationAspect, VideoGenerationError,
    VideoGenerationRequest, VideoGenerationResponse, VideoGenerationResult, VideoGenerator,
};
use async_trait::async_trait;

use super::{
    client::OpenAiVideoGenerator,
    request::build_request,
    response::{JobState, parse_job},
};

#[async_trait]
impl VideoGenerator for OpenAiVideoGenerator {
    async fn generate(
        &self,
        request: &VideoGenerationRequest,
    ) -> VideoGenerationResult<VideoGenerationResponse> {
        let context = self.context();
        let api_request = build_request(&context, &self.model_id, request)?;
        let started_at = self.runtime.now();
        let response = self
            .send_job_request(&context, api_request, self.remaining(&context, started_at)?)
            .await?;
        let mut response_context = context.for_response(&response.headers);
        let (job_id, mut state) = parse_job(&response_context, response.body)?;
        while state == JobState::Pending {
            let remaining = self.remaining(&response_context, started_at)?;
            self.runtime.sleep(remaining.min(self.poll_interval)).await;
            let response = self
                .retrieve_job(
                    &context,
                    &job_id,
                    self.remaining(&response_context, started_at)?,
                )
                .await?;
            response_context = context.for_response(&response.headers);
            let (retrieved_id, retrieved_state) = parse_job(&response_context, response.body)?;
            if retrieved_id != job_id {
                return Err(VideoGenerationError::new(response_context.core(
                    ErrorKind::InvalidOutput,
                    "provider changed the video job id while polling",
                )));
            }
            state = retrieved_state;
        }
        let data = self
            .download(
                &context,
                &job_id,
                self.remaining(&response_context, started_at)?,
            )
            .await?;
        let (width, height) = dimensions(request.aspect);
        Ok(VideoGenerationResponse {
            provider: "openai".to_owned(),
            model_id: self.model_id.clone(),
            video: GeneratedVideo {
                data,
                mime_type: "video/mp4".to_owned(),
                duration_seconds: request.duration.seconds(),
                width,
                height,
            },
            usage: ModelUsage::default(),
        })
    }
}

fn dimensions(aspect: VideoGenerationAspect) -> (u16, u16) {
    match aspect {
        VideoGenerationAspect::Landscape => (1280, 720),
        VideoGenerationAspect::Portrait => (720, 1280),
    }
}
