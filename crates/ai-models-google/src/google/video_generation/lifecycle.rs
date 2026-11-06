//! Google operation polling retains only the relevant response facts.

use ai_interface::{
    ErrorKind, GeneratedVideo, ModelUsage, VideoGenerationAspect, VideoGenerationError,
    VideoGenerationRequest, VideoGenerationResponse, VideoGenerationResult, VideoGenerator,
};
use async_trait::async_trait;

use super::{
    client::GoogleVideoGenerator,
    deadline::VideoDeadline,
    response::{OperationState, parse_operation},
};

#[async_trait]
impl VideoGenerator for GoogleVideoGenerator {
    async fn generate(
        &self,
        request: &VideoGenerationRequest,
    ) -> VideoGenerationResult<VideoGenerationResponse> {
        let context = self.context();
        let deadline = VideoDeadline::new(self.runtime.as_ref(), self.timeout);
        let response = self.submit(&context, request, &deadline).await?;
        let mut response_context = context.for_response(&response.headers);
        let (operation, mut state) =
            parse_operation(&response_context, &response.headers, response.body)?;
        while state == OperationState::Pending {
            let remaining = deadline.remaining(&response_context)?;
            self.runtime.sleep(remaining.min(self.poll_interval)).await;
            deadline.remaining(&response_context)?;
            let response = self.retrieve(&context, &operation, &deadline).await?;
            response_context = context.for_response(&response.headers);
            let (retrieved_operation, retrieved_state) =
                parse_operation(&response_context, &response.headers, response.body)?;
            if retrieved_operation != operation {
                return Err(VideoGenerationError::new(response_context.core(
                    ErrorKind::InvalidOutput,
                    "provider changed the operation name while polling",
                )));
            }
            state = retrieved_state;
        }
        let OperationState::Completed { download_uri } = state else {
            return Err(VideoGenerationError::NoVideo {
                core: Box::new(
                    response_context.core(ErrorKind::InvalidOutput, "no video returned"),
                ),
            });
        };
        let data = self
            .download(&response_context, &download_uri, &deadline)
            .await?;
        let (width, height) = dimensions(request.aspect);
        Ok(VideoGenerationResponse {
            provider: "google".to_owned(),
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
