//! Normalized live-image response validation tests.

use ai_interface::{GeneratedImage, ImageGenerationResponse, ModelUsage, ProviderKind};
use ai_models_core::{
    CostTier, IntelligenceScore, KnownModelSpec, ModelFeature, SpeedTier, ThinkingLevel,
};

const IMAGE_FEATURES: &[ModelFeature] = &[ModelFeature::ImageGeneration];

pub(super) fn validation_failures(
    spec: &KnownModelSpec,
    response: &ImageGenerationResponse,
) -> Vec<String> {
    let mut failures = Vec::new();
    let expected_provider = spec.provider.as_str();

    if response.provider != expected_provider {
        failures.push(format!(
            "{}: provider was `{}`, expected `{expected_provider}`",
            spec.id, response.provider
        ));
    }
    if response.model_id != spec.provider_model_id {
        failures.push(format!(
            "{}: provider model was `{}`, expected `{}`",
            spec.id, response.model_id, spec.provider_model_id
        ));
    }
    if response.image.data.is_empty() {
        failures.push(format!("{}: image bytes were empty", spec.id));
        return failures;
    }

    let signature_matches = match response.image.mime_type.as_str() {
        "image/png" => response.image.data.starts_with(b"\x89PNG\r\n\x1a\n"),
        "image/jpeg" => response.image.data.starts_with(b"\xff\xd8\xff"),
        "image/webp" => {
            response.image.data.len() >= 12
                && response.image.data.starts_with(b"RIFF")
                && &response.image.data[8..12] == b"WEBP"
        }
        unsupported => {
            failures.push(format!(
                "{}: unsupported MIME type `{unsupported}`",
                spec.id
            ));
            return failures;
        }
    };
    if !signature_matches {
        failures.push(format!(
            "{}: image signature did not match MIME type `{}`",
            spec.id, response.image.mime_type
        ));
    }

    failures
}

#[test]
fn accepts_supported_mime_types_with_matching_file_signatures() {
    for (mime_type, data) in [
        ("image/png", b"\x89PNG\r\n\x1a\nrest".as_slice()),
        ("image/jpeg", b"\xff\xd8\xffrest".as_slice()),
        ("image/webp", b"RIFF\x04\x00\x00\x00WEBPrest".as_slice()),
    ] {
        assert_eq!(
            validation_failures(
                &image_spec(),
                &response("openai", "gpt-image-2", mime_type, data),
            ),
            Vec::<String>::new()
        );
    }
}

#[test]
fn reports_provider_and_model_identity_mismatches() {
    let failures = validation_failures(
        &image_spec(),
        &response(
            "google",
            "gemini-3.1-flash-image",
            "image/png",
            b"\x89PNG\r\n\x1a\n",
        ),
    );

    assert_eq!(failures.len(), 2);
    assert!(failures.iter().any(|failure| failure.contains("provider")));
    assert!(failures.iter().any(|failure| failure.contains("model")));
}

#[test]
fn rejects_empty_image_bytes() {
    let failures = validation_failures(
        &image_spec(),
        &response("openai", "gpt-image-2", "image/png", b""),
    );

    assert!(failures.iter().any(|failure| failure.contains("empty")));
}

#[test]
fn rejects_unsupported_mime_types() {
    let failures = validation_failures(
        &image_spec(),
        &response("openai", "gpt-image-2", "image/gif", b"GIF89a"),
    );

    assert!(
        failures
            .iter()
            .any(|failure| failure.contains("unsupported MIME type"))
    );
}

#[test]
fn rejects_mime_and_file_signature_disagreement() {
    let failures = validation_failures(
        &image_spec(),
        &response("openai", "gpt-image-2", "image/png", b"\xff\xd8\xffrest"),
    );

    assert!(failures.iter().any(|failure| failure.contains("signature")));
}

fn image_spec() -> KnownModelSpec {
    KnownModelSpec {
        provider: ProviderKind::OpenAi,
        id: "image-catalog-id",
        provider_model_id: "gpt-image-2",
        context_window_tokens: 0,
        intelligence_score: IntelligenceScore::Ten,
        speed: SpeedTier::Medium,
        cost: CostTier::Premium,
        thinking_level: ThinkingLevel::Disabled,
        features: IMAGE_FEATURES,
    }
}

fn response(
    provider: &str,
    model_id: &str,
    mime_type: &str,
    data: &[u8],
) -> ImageGenerationResponse {
    ImageGenerationResponse {
        provider: provider.to_owned(),
        model_id: model_id.to_owned(),
        image: GeneratedImage {
            data: data.to_vec(),
            mime_type: mime_type.to_owned(),
        },
        revised_prompt: None,
        usage: ModelUsage::default(),
    }
}
