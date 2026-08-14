//! Catalog iteration, credential handling, and probe request tests.

use std::{env, sync::Arc};

use ai_interface::{
    DynVideoGenerator, VideoGenerationAspect, VideoGenerationDuration, VideoGenerationRequest,
    VideoGenerationResolution,
};
use json_http::{JsonHttpClient, ReqwestJsonHttpClient};
use thiserror::Error;

use super::{provider_tests::LiveVideoProvider, validation_tests::validation_failures};

const API_KEY_ENV: &str = "LIVE_VIDEO_API_KEY";

#[derive(Debug, Eq, Error, PartialEq)]
enum ApiKeyError {
    #[error("[xtask/live_videos] LIVE_VIDEO_API_KEY is missing")]
    Missing,
    #[error("[xtask/live_videos] LIVE_VIDEO_API_KEY is blank")]
    Blank,
}

pub(super) async fn run_catalog(provider: LiveVideoProvider) {
    let api_key =
        require_api_key(env::var(API_KEY_ENV).ok()).unwrap_or_else(|error| panic!("{error}"));
    let client: Arc<dyn JsonHttpClient> = Arc::new(ReqwestJsonHttpClient::new());
    let auth = provider.auth(api_key);
    let catalog = provider.catalog();
    assert!(
        !catalog.is_empty(),
        "{provider:?} video catalog must not be empty"
    );
    let mut failures = Vec::new();
    for spec in catalog {
        println!("checking {}/{}", spec.provider, spec.id);
        let generator: DynVideoGenerator = provider.build(client.clone(), auth.clone(), &spec);
        match generator.generate(&probe_request()).await {
            Ok(response) => failures.extend(validation_failures(&spec, &response)),
            Err(error) => failures.push(format!("{}: request failed: {error}", spec.id)),
        }
    }
    assert!(
        failures.is_empty(),
        "{} live video catalog failure(s):\n{}",
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

fn probe_request() -> VideoGenerationRequest {
    VideoGenerationRequest {
        prompt: "A video of the words 'Thank you' in sparkling letters".to_owned(),
        input_image: None,
        aspect: VideoGenerationAspect::Landscape,
        duration: VideoGenerationDuration::Seconds4,
        resolution: VideoGenerationResolution::P720,
    }
}

#[test]
fn requires_a_present_non_empty_api_key() {
    assert!(matches!(require_api_key(None), Err(ApiKeyError::Missing)));
    assert!(matches!(
        require_api_key(Some(" \n\t".to_owned())),
        Err(ApiKeyError::Blank)
    ));
    assert_eq!(require_api_key(Some(" key ".to_owned())).unwrap(), " key ");
}

#[test]
fn probe_is_safe_portable_and_minimal() {
    let request = probe_request();
    assert_eq!(
        request.prompt,
        "A video of the words 'Thank you' in sparkling letters"
    );
    assert!(request.input_image.is_none());
    assert_eq!(request.aspect, VideoGenerationAspect::Landscape);
    assert_eq!(request.duration, VideoGenerationDuration::Seconds4);
    assert_eq!(request.resolution, VideoGenerationResolution::P720);
}
