//! Typed adapter construction failures without credentials or transport diagnostics.

use ai_interface::ProviderId;
use thiserror::Error;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
/// Adapter boundary requested from a provider.
pub enum Modality {
    /// A streaming conversational model.
    Model,
    /// An image generator or editor.
    Image,
    /// An audio transcriber.
    Transcription,
    /// A typed judgment model.
    Judgment,
}

#[derive(Clone, Debug, Eq, Error, PartialEq)]
/// Failures that prevent the registry from building an adapter.
pub enum BuildError {
    /// The provider has no stored configuration.
    #[error("[ai_models_registry/error] provider `{provider}` has no configuration")]
    MissingConfig {
        /// Provider selected by the model spec.
        provider: ProviderId,
    },
    /// No enabled feature includes this provider.
    #[error("[ai_models_registry/error] provider `{provider}` is not included")]
    ProviderNotIncluded {
        /// Provider selected by the model spec.
        provider: ProviderId,
    },
    /// The included provider has no adapter of the requested modality.
    #[error("[ai_models_registry/error] provider `{provider}` has no {modality:?} adapter")]
    UnsupportedModality {
        /// Provider selected by the model spec.
        provider: ProviderId,
        /// Requested adapter boundary.
        modality: Modality,
    },
}

/// Result of constructing one adapter from a model spec.
pub type BuildResult<T> = Result<T, BuildError>;
