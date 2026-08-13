//! Image-generation retry behavior tests.

use std::{
    collections::VecDeque,
    sync::{
        Arc, Mutex,
        atomic::{AtomicUsize, Ordering},
    },
    time::Duration,
};

use ai_interface::{
    DynImageGenerator, GeneratedImage, ImageGenerationError, ImageGenerationRequest,
    ImageGenerationResponse, ImageGenerator, ImageGeneratorMock, ModelUsage,
};
use ai_models_core::{DynSleeper, STANDARD_TRANSIENT_RETRY_DELAYS, TokioSleeper};
use async_trait::async_trait;
use unimock::{MockFn, Unimock, matching};

#[derive(Clone)]
pub(super) struct RetryingImageGenerator {
    inner: DynImageGenerator,
    sleeper: DynSleeper,
    retry_delays: Vec<Duration>,
}

impl RetryingImageGenerator {
    pub(super) fn new(
        inner: DynImageGenerator,
        sleeper: DynSleeper,
        retry_delays: Vec<Duration>,
    ) -> Self {
        Self {
            inner,
            sleeper,
            retry_delays,
        }
    }

    pub(super) fn with_standard_transient_retry(inner: DynImageGenerator) -> Self {
        Self::new(
            inner,
            Arc::new(TokioSleeper),
            Self::standard_retry_delays().to_vec(),
        )
    }

    pub(super) fn standard_retry_delays() -> [Duration; 2] {
        STANDARD_TRANSIENT_RETRY_DELAYS
    }
}

#[async_trait]
impl ImageGenerator for RetryingImageGenerator {
    async fn generate(
        &self,
        request: &ImageGenerationRequest,
    ) -> Result<ImageGenerationResponse, ImageGenerationError> {
        let mut retry_index = 0usize;

        loop {
            match self.inner.generate(request).await {
                Err(
                    ImageGenerationError::RateLimited { .. }
                    | ImageGenerationError::TransientProvider { .. },
                ) if retry_index < self.retry_delays.len() => {
                    let delay = self.retry_delays[retry_index];
                    retry_index += 1;
                    self.sleeper.sleep(delay).await;
                }
                result => return result,
            }
        }
    }
}

#[tokio::test]
async fn retries_only_rate_limits_and_transient_provider_failures() {
    let calls = Arc::new(AtomicUsize::new(0));
    let retrying = RetryingImageGenerator::new(
        scripted_generator(
            vec![
                Err(ImageGenerationError::rate_limited(
                    "openai",
                    "gpt-image-2",
                    "retry one",
                )),
                Err(ImageGenerationError::transient_provider(
                    "openai",
                    "gpt-image-2",
                    "retry two",
                )),
                Ok(success_response()),
            ],
            calls.clone(),
        ),
        Arc::new(TokioSleeper),
        vec![Duration::ZERO, Duration::ZERO],
    );

    retrying
        .generate(&probe_request())
        .await
        .expect("third attempt should succeed");

    assert_eq!(calls.load(Ordering::SeqCst), 3);
}

#[tokio::test]
async fn returns_the_last_retryable_error_after_three_attempts() {
    let calls = Arc::new(AtomicUsize::new(0));
    let retrying = RetryingImageGenerator::new(
        scripted_generator(
            vec![transient("one"), transient("two"), transient("three")],
            calls.clone(),
        ),
        Arc::new(TokioSleeper),
        vec![Duration::ZERO, Duration::ZERO],
    );

    let error = retrying
        .generate(&probe_request())
        .await
        .expect_err("third transient failure should be returned");

    assert!(matches!(
        error,
        ImageGenerationError::TransientProvider { .. }
    ));
    assert_eq!(calls.load(Ordering::SeqCst), 3);
}

#[tokio::test]
async fn terminal_error_classes_stop_after_one_attempt() {
    let errors = vec![
        ImageGenerationError::EmptyPrompt,
        ImageGenerationError::unsupported_media_type("image/gif"),
        ImageGenerationError::content_policy("openai", "gpt-image-2", "blocked"),
        ImageGenerationError::no_image("openai", "gpt-image-2"),
        ImageGenerationError::provider("openai", "gpt-image-2", "rejected"),
        ImageGenerationError::internal(std::io::Error::other("internal")),
    ];

    for error in errors {
        let calls = Arc::new(AtomicUsize::new(0));
        let retrying = RetryingImageGenerator::new(
            scripted_generator(vec![Err(error)], calls.clone()),
            Arc::new(TokioSleeper),
            vec![Duration::ZERO, Duration::ZERO],
        );

        retrying
            .generate(&probe_request())
            .await
            .expect_err("terminal error should be returned");
        assert_eq!(calls.load(Ordering::SeqCst), 1);
    }
}

#[test]
fn standard_retry_schedule_is_the_shared_bounded_schedule() {
    assert_eq!(
        RetryingImageGenerator::standard_retry_delays(),
        STANDARD_TRANSIENT_RETRY_DELAYS
    );
}

fn scripted_generator(
    responses: Vec<Result<ImageGenerationResponse, ImageGenerationError>>,
    calls: Arc<AtomicUsize>,
) -> DynImageGenerator {
    let responses = Arc::new(Mutex::new(VecDeque::from(responses)));
    Arc::new(Unimock::new(
        ImageGeneratorMock::generate
            .each_call(matching!(_))
            .answers_arc({
                let responses = responses.clone();
                Arc::new(move |_, _request: &ImageGenerationRequest| {
                    calls.fetch_add(1, Ordering::SeqCst);
                    responses
                        .lock()
                        .expect("response lock should not be poisoned")
                        .pop_front()
                        .expect("unexpected image generation call")
                })
            }),
    ))
}

fn transient(message: &str) -> Result<ImageGenerationResponse, ImageGenerationError> {
    Err(ImageGenerationError::transient_provider(
        "openai",
        "gpt-image-2",
        message,
    ))
}

fn probe_request() -> ImageGenerationRequest {
    ImageGenerationRequest {
        prompt: "A blue circle".to_owned(),
        ..ImageGenerationRequest::default()
    }
}

fn success_response() -> ImageGenerationResponse {
    ImageGenerationResponse {
        provider: "openai".to_owned(),
        model_id: "gpt-image-2".to_owned(),
        image: GeneratedImage {
            data: b"\x89PNG\r\n\x1a\n".to_vec(),
            mime_type: "image/png".to_owned(),
        },
        revised_prompt: None,
        usage: ModelUsage::default(),
    }
}
