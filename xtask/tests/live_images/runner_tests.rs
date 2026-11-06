//! Catalog iteration, credential handling, and probe request tests.

use std::{
    borrow::Cow,
    env,
    sync::{Arc, Mutex},
};

use ai_interface::{
    DynImageGenerator, ErrorCore, ErrorKind, ImageGenerationAspect, ImageGenerationError,
    ImageGenerationQuality, ImageGenerationRequest, ImageGeneratorMock, MockImageGenerator,
    ModelFeature, ModelSpec,
};
use json_http::{JsonHttpClient, ReqwestJsonHttpClient};
use thiserror::Error;
use unimock::{MockFn, Unimock, matching};

use super::{
    provider_tests::LiveImageProvider, retry_tests::RetryingImageGenerator,
    validation_tests::validation_failures,
};

const IMAGE_FEATURES: &[ModelFeature] = &[ModelFeature::ImageGeneration];
const API_KEY_ENV: &str = "LIVE_IMAGE_API_KEY";

#[derive(Debug, Eq, Error, PartialEq)]
enum ApiKeyError {
    #[error("[xtask/live_images] LIVE_IMAGE_API_KEY is missing")]
    Missing,
    #[error("[xtask/live_images] LIVE_IMAGE_API_KEY is blank")]
    Blank,
}

struct ProbeTarget {
    spec: ModelSpec,
    generator: DynImageGenerator,
}

pub(super) async fn run_catalog(provider: LiveImageProvider) {
    let api_key =
        require_api_key(env::var(API_KEY_ENV).ok()).unwrap_or_else(|error| panic!("{error}"));
    let client: Arc<dyn JsonHttpClient> = Arc::new(ReqwestJsonHttpClient::new());
    let auth = provider.auth(api_key);
    let catalog = provider.image_catalog();
    assert!(
        !catalog.is_empty(),
        "{provider:?} image catalog must not be empty"
    );
    let targets = catalog
        .into_iter()
        .map(|spec| {
            let inner = provider.build(client.clone(), auth.clone(), &spec);
            ProbeTarget {
                spec,
                generator: Arc::new(RetryingImageGenerator::with_standard_transient_retry(inner)),
            }
        })
        .collect();
    let failures = run_targets(targets).await;

    assert!(
        failures.is_empty(),
        "{} live image catalog failure(s):\n{}",
        failures.len(),
        failures.join("\n")
    );
}

fn require_api_key(value: Option<String>) -> Result<String, ApiKeyError> {
    let value = value.ok_or(ApiKeyError::Missing)?;
    if value.trim().is_empty() {
        return Err(ApiKeyError::Blank);
    }
    Ok(value)
}

fn probe_request() -> ImageGenerationRequest {
    ImageGenerationRequest {
        prompt: "A simple solid blue circle on a plain white background.".to_owned(),
        input_images: Vec::new(),
        aspect: ImageGenerationAspect::Square,
        quality: ImageGenerationQuality::Low,
    }
}

async fn run_targets(targets: Vec<ProbeTarget>) -> Vec<String> {
    let request = probe_request();
    let mut failures = Vec::new();

    for target in targets {
        println!("{}", check_message(&target.spec));
        match target.generator.generate(&request).await {
            Ok(response) => failures.extend(validation_failures(&target.spec, &response)),
            Err(error) => failures.push(request_failure_message(&target.spec, &error)),
        }
    }

    failures
}

fn check_message(spec: &ModelSpec) -> String {
    format!("checking {}", spec.text_id())
}

fn request_failure_message(spec: &ModelSpec, error: &ImageGenerationError) -> String {
    format!("{}: request failed: {error}", spec.text_id())
}

#[test]
fn requires_a_present_non_empty_api_key() {
    assert!(matches!(require_api_key(None), Err(ApiKeyError::Missing)));
    assert!(matches!(
        require_api_key(Some(" \n\t".to_owned())),
        Err(ApiKeyError::Blank)
    ));
    assert_eq!(
        require_api_key(Some(" secret ".to_owned())).unwrap(),
        " secret "
    );
}

#[test]
fn probe_request_is_safe_provider_neutral_and_low_cost() {
    let request = probe_request();

    assert_eq!(
        request.prompt,
        "A simple solid blue circle on a plain white background."
    );
    assert!(request.input_images.is_empty());
    assert_eq!(request.aspect, ImageGenerationAspect::Square);
    assert_eq!(request.quality, ImageGenerationQuality::Low);
}

#[tokio::test]
async fn runs_targets_sequentially_and_aggregates_every_failure() {
    let calls = Arc::new(Mutex::new(Vec::new()));
    let targets = vec![
        target("first", recording_failure_generator("first", calls.clone())),
        target(
            "second",
            recording_failure_generator("second", calls.clone()),
        ),
    ];

    let failures = run_targets(targets).await;

    assert_eq!(
        *calls.lock().expect("call lock should not be poisoned"),
        vec!["first", "second"]
    );
    assert_eq!(failures.len(), 2);
    assert!(failures.iter().any(|failure| failure.contains("first")));
    assert!(failures.iter().any(|failure| failure.contains("second")));
}

#[tokio::test]
async fn runner_accepts_a_provider_neutral_dynamic_generator() {
    let failures = run_targets(vec![ProbeTarget {
        spec: mock_spec(),
        generator: Arc::new(MockImageGenerator::default()),
    }])
    .await;

    assert!(failures.is_empty());
}

#[test]
fn log_messages_contain_identifiers_but_not_sensitive_values() {
    let spec = image_spec("gpt-image-2");
    let check = check_message(&spec);
    let failure = request_failure_message(
        &spec,
        &ImageGenerationError::new(ErrorCore::new(
            ErrorKind::InvalidRequest,
            "openai",
            "gpt-image-2",
            "rejected",
        )),
    );

    assert_eq!(check, "checking openai/gpt-image-2");
    assert!(failure.contains("openai/gpt-image-2"));
    assert!(!check.contains("secret-key"));
    assert!(!failure.contains("[137, 80, 78, 71"));
}

fn recording_failure_generator(
    label: &'static str,
    calls: Arc<Mutex<Vec<&'static str>>>,
) -> DynImageGenerator {
    Arc::new(Unimock::new(
        ImageGeneratorMock::generate
            .next_call(matching!(_))
            .answers_arc(Arc::new(move |_, _request: &ImageGenerationRequest| {
                calls
                    .lock()
                    .expect("call lock should not be poisoned")
                    .push(label);
                Err(ImageGenerationError::new(ErrorCore::new(
                    ErrorKind::InvalidRequest,
                    "openai",
                    label,
                    "expected failure",
                )))
            })),
    ))
}

fn target(id: &'static str, generator: DynImageGenerator) -> ProbeTarget {
    ProbeTarget {
        spec: image_spec(id),
        generator,
    }
}

fn image_spec(provider_model_id: &'static str) -> ModelSpec {
    ModelSpec {
        features: Cow::Borrowed(IMAGE_FEATURES),
        ..ModelSpec::custom(
            ai_models_openai::PROVIDER_ID,
            provider_model_id,
            &ai_models_openai::CUSTOM_DEFAULTS,
        )
    }
}

fn mock_spec() -> ModelSpec {
    ModelSpec {
        features: Cow::Borrowed(IMAGE_FEATURES),
        ..ModelSpec::custom(
            ai_interface::MOCK_PROVIDER,
            "mock-image",
            &ai_interface::MOCK_CUSTOM_DEFAULTS,
        )
    }
}
