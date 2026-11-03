//! Feature-selected image, transcription, judgment, and video factories.

#[cfg(any(feature = "google", feature = "openai", feature = "typesafe"))]
use std::sync::Arc;

use ai_interface::{
    DynAudioTranscriber, DynImageGenerator, DynJudgmentModel, DynVideoGenerator, ModelSpec,
};
use json_http::DynJsonHttpClient;

use crate::{BuildError, BuildResult, Modality, ProviderConfig};

/// Builds the image adapter of an included, configured provider.
pub(crate) fn image(
    http: DynJsonHttpClient,
    spec: &ModelSpec,
    config: ProviderConfig,
) -> BuildResult<DynImageGenerator> {
    match (spec.provider.as_str(), http, config) {
        #[cfg(feature = "google")]
        ("google", http, config) => Ok(Arc::new(
            ai_models_google::GoogleImageGenerator::from_config(http, spec.clone(), config),
        )),
        #[cfg(feature = "openai")]
        ("openai", http, config) => Ok(Arc::new(
            ai_models_openai::OpenAiImageGenerator::from_config(http, spec.clone(), config),
        )),
        _ => Err(BuildError::UnsupportedModality {
            provider: spec.provider.clone(),
            modality: Modality::Image,
        }),
    }
}

/// Builds the transcription adapter of an included, configured provider.
pub(crate) fn transcription(
    http: DynJsonHttpClient,
    spec: &ModelSpec,
    config: ProviderConfig,
) -> BuildResult<DynAudioTranscriber> {
    match (spec.provider.as_str(), http, config) {
        #[cfg(feature = "openai")]
        ("openai", http, config) => Ok(Arc::new(
            ai_models_openai::OpenAiAudioTranscriber::from_config(http, spec.clone(), config),
        )),
        _ => Err(BuildError::UnsupportedModality {
            provider: spec.provider.clone(),
            modality: Modality::Transcription,
        }),
    }
}

/// Builds the judgment adapter of an included, configured provider.
pub(crate) fn judgment(
    http: DynJsonHttpClient,
    spec: &ModelSpec,
    config: ProviderConfig,
) -> BuildResult<DynJudgmentModel> {
    match (spec.provider.as_str(), http, config) {
        #[cfg(feature = "typesafe")]
        ("typesafe", http, config) => Ok(Arc::new(
            ai_models_typesafe::TypeSafeJudgmentModel::from_config(http, spec.clone(), config),
        )),
        _ => Err(BuildError::UnsupportedModality {
            provider: spec.provider.clone(),
            modality: Modality::Judgment,
        }),
    }
}

/// Builds the video adapter of an included, configured provider.
pub(crate) fn video(
    http: DynJsonHttpClient,
    spec: &ModelSpec,
    config: ProviderConfig,
) -> BuildResult<DynVideoGenerator> {
    match (spec.provider.as_str(), http, config) {
        #[cfg(feature = "google")]
        ("google", http, config) => Ok(Arc::new(
            ai_models_google::GoogleVideoGenerator::from_config(http, spec.clone(), config),
        )),
        _ => Err(BuildError::UnsupportedModality {
            provider: spec.provider.clone(),
            modality: Modality::Video,
        }),
    }
}
