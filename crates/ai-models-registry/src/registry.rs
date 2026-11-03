//! Shared configuration ownership and adapter construction entry points.

use std::collections::BTreeMap;

use ai_interface::{
    DynAudioTranscriber, DynImageGenerator, DynJudgmentModel, DynModel, DynVideoGenerator,
    LegacyMessageConverter, ModelCatalog, ModelSpec, ProviderId,
};
use json_http::DynJsonHttpClient;

use crate::{BuildError, BuildResult, ProviderConfig, catalog, modalities, models, providers};

#[derive(Clone)]
/// Builds included provider adapters with one HTTP client and one config per provider.
pub struct Registry {
    http_client: DynJsonHttpClient,
    configs: BTreeMap<ProviderId, ProviderConfig>,
}

impl Registry {
    /// Builds an empty registry without transport or credential resolution.
    pub fn new(http_client: DynJsonHttpClient) -> Self {
        Self {
            http_client,
            configs: BTreeMap::new(),
        }
    }

    /// Stores provider settings, replacing any earlier settings for that provider.
    pub fn with_provider(mut self, config: ProviderConfig) -> Self {
        self.insert(config);
        self
    }

    /// Stores provider settings and returns the settings that they replace.
    pub fn insert(&mut self, config: ProviderConfig) -> Option<ProviderConfig> {
        self.configs.insert(config.provider.clone(), config)
    }

    /// Builds a streaming model from its complete model spec.
    pub fn model(&self, spec: &ModelSpec) -> BuildResult<DynModel> {
        let config = self.configured(&spec.provider)?;
        models::build(self.http_client.clone(), spec, config)
    }

    /// Builds an image generator from its model spec.
    pub fn image(&self, spec: &ModelSpec) -> BuildResult<DynImageGenerator> {
        let config = self.configured(&spec.provider)?;
        modalities::image(self.http_client.clone(), spec, config)
    }

    /// Builds an audio transcriber from its model spec.
    pub fn transcription(&self, spec: &ModelSpec) -> BuildResult<DynAudioTranscriber> {
        let config = self.configured(&spec.provider)?;
        modalities::transcription(self.http_client.clone(), spec, config)
    }

    /// Builds a typed judgment model from its model spec.
    pub fn judgment(&self, spec: &ModelSpec) -> BuildResult<DynJudgmentModel> {
        let config = self.configured(&spec.provider)?;
        modalities::judgment(self.http_client.clone(), spec, config)
    }

    /// Builds a video generator from its model spec.
    pub fn video(&self, spec: &ModelSpec) -> BuildResult<DynVideoGenerator> {
        let config = self.configured(&spec.provider)?;
        modalities::video(self.http_client.clone(), spec, config)
    }

    /// Returns the combined catalogs of the included providers.
    pub fn catalog(&self) -> ModelCatalog {
        catalog()
    }

    /// Composes the stored-message converters of the included providers.
    pub fn converter(&self) -> LegacyMessageConverter {
        providers::converter()
    }

    /// Checks provider inclusion before config lookup without applying credentials.
    fn configured(&self, provider: &ProviderId) -> BuildResult<ProviderConfig> {
        if !providers::INCLUDED.contains(provider) {
            return Err(BuildError::ProviderNotIncluded {
                provider: provider.clone(),
            });
        }
        match self.configs.get(provider) {
            Some(config) => Ok(config.clone()),
            None => Err(BuildError::MissingConfig {
                provider: provider.clone(),
            }),
        }
    }
}
