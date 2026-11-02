//! Exact build failures, config replacement, and construction without side effects.

use std::{sync::Arc, time::Duration};

use ai_interface::ModelSpec;
use unimock::Unimock;

use crate::{BuildError, Modality, ProviderConfig, Registry};

use super::support::{PROVIDERS, spec, unused_http};

#[test]
fn builds_supported_adapters_without_transport_or_auth() {
    for (provider, enabled, supported) in PROVIDERS.iter().filter(|(_, enabled, _)| *enabled) {
        let spec = spec(provider);
        let registry = Registry::new(unused_http()).with_provider(ProviderConfig::with_auth(
            spec.provider.clone(),
            Arc::new(Unimock::new(())),
        ));
        let built = [
            registry.model(&spec).is_ok(),
            registry.image(&spec).is_ok(),
            registry.transcription(&spec).is_ok(),
            registry.judgment(&spec).is_ok(),
        ];
        assert_eq!(built, *supported, "{provider}, enabled={enabled}");
    }
}

#[test]
fn reports_typed_build_errors_for_all_provider_feature_states() {
    for (provider, enabled, supported) in PROVIDERS {
        let spec = spec(provider);
        let absent = Registry::new(unused_http());
        let configured = Registry::new(unused_http())
            .with_provider(ProviderConfig::new(spec.provider.clone(), "registry-key"));
        for ((modality, absent_error), (_, configured_error), supports) in errors(&absent, &spec)
            .into_iter()
            .zip(errors(&configured, &spec))
            .zip(supported)
            .map(|((absent, configured), supports)| (absent, configured, supports))
        {
            let expected_absent = if *enabled {
                BuildError::MissingConfig {
                    provider: spec.provider.clone(),
                }
            } else {
                BuildError::ProviderNotIncluded {
                    provider: spec.provider.clone(),
                }
            };
            assert_eq!(
                absent_error,
                Some(expected_absent),
                "{provider} {modality:?}"
            );
            let expected_configured = if !enabled {
                Some(BuildError::ProviderNotIncluded {
                    provider: spec.provider.clone(),
                })
            } else if !supports {
                Some(BuildError::UnsupportedModality {
                    provider: spec.provider.clone(),
                    modality,
                })
            } else {
                None
            };
            assert_eq!(
                configured_error, expected_configured,
                "{provider} {modality:?}"
            );
        }
    }
}

#[test]
fn insert_replaces_configuration_for_one_provider() {
    let spec = spec("custom-provider");
    let first = ProviderConfig::new(spec.provider.clone(), "first-key")
        .with_header("X-Tenant", "first")
        .with_total_timeout(Duration::from_secs(17));
    let second = ProviderConfig::new(spec.provider.clone(), "second-key")
        .with_header("X-Tenant", "second")
        .with_total_timeout(Duration::from_secs(29));
    let mut registry = Registry::new(unused_http()).with_provider(first);

    let previous = registry.insert(second);

    assert_eq!(
        previous
            .as_ref()
            .and_then(|config| config.extra_headers.get("X-Tenant"))
            .map(String::as_str),
        Some("first")
    );
    assert_eq!(
        previous
            .as_ref()
            .and_then(|config| config.timeouts.total_timeout),
        Some(Duration::from_secs(17))
    );
    let previous = registry.insert(ProviderConfig::new(spec.provider, "third-key"));
    assert_eq!(
        previous
            .as_ref()
            .and_then(|config| config.extra_headers.get("X-Tenant"))
            .map(String::as_str),
        Some("second")
    );
    assert_eq!(
        previous
            .as_ref()
            .and_then(|config| config.timeouts.total_timeout),
        Some(Duration::from_secs(29))
    );
}

fn errors(registry: &Registry, spec: &ModelSpec) -> [(Modality, Option<BuildError>); 4] {
    [
        (Modality::Model, registry.model(spec).err()),
        (Modality::Image, registry.image(spec).err()),
        (Modality::Transcription, registry.transcription(spec).err()),
        (Modality::Judgment, registry.judgment(spec).err()),
    ]
}
