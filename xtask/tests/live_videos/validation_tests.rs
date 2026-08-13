//! Normalized live-video response validation tests.

use ai_interface::{GeneratedVideo, ModelUsage, ProviderKind, VideoGenerationResponse};
use ai_models_core::{
    CostTier, IntelligenceScore, KnownModelSpec, ModelFeature, SpeedTier, ThinkingLevel,
};

const VIDEO_FEATURES: &[ModelFeature] = &[ModelFeature::VideoGeneration];

pub(super) fn validation_failures(
    spec: &KnownModelSpec,
    response: &VideoGenerationResponse,
) -> Vec<String> {
    let mut failures = Vec::new();
    if response.provider != spec.provider.as_str() {
        failures.push(format!("{}: provider identity mismatch", spec.id));
    }
    if response.model_id != spec.provider_model_id {
        failures.push(format!("{}: provider model identity mismatch", spec.id));
    }
    if response.video.data.is_empty() {
        failures.push(format!("{}: video bytes were empty", spec.id));
        return failures;
    }
    if response.video.mime_type != "video/mp4" {
        failures.push(format!("{}: video MIME was not video/mp4", spec.id));
    }
    if response.video.data.len() < 8 || &response.video.data[4..8] != b"ftyp" {
        failures.push(format!("{}: video did not have an MP4 signature", spec.id));
    }
    if response.video.duration_seconds != 4
        || (response.video.width, response.video.height) != (1280, 720)
    {
        failures.push(format!(
            "{}: normalized video metadata was incorrect",
            spec.id
        ));
    }
    failures
}

#[test]
fn accepts_expected_mp4_and_metadata() {
    assert_eq!(
        validation_failures(&video_spec(), &response("openai", "sora-2", mp4())),
        Vec::<String>::new()
    );
}

#[test]
fn reports_identity_payload_and_metadata_failures() {
    let mut response = response("google", "veo", b"not-mp4");
    response.video.mime_type = "video/webm".to_owned();
    response.video.duration_seconds = 8;

    let failures = validation_failures(&video_spec(), &response);

    assert_eq!(failures.len(), 5);
    for expected in ["provider", "model", "MIME", "signature", "metadata"] {
        assert!(
            failures.iter().any(|failure| failure.contains(expected)),
            "missing `{expected}` validation failure"
        );
    }
}

#[test]
fn rejects_empty_video_bytes() {
    let failures = validation_failures(&video_spec(), &response("openai", "sora-2", b""));

    assert_eq!(failures.len(), 1);
    assert!(failures[0].contains("empty"));
}

fn video_spec() -> KnownModelSpec {
    KnownModelSpec {
        provider: ProviderKind::OpenAi,
        id: "video-catalog-id",
        provider_model_id: "sora-2",
        context_window_tokens: 0,
        intelligence_score: IntelligenceScore::Ten,
        speed: SpeedTier::Slow,
        cost: CostTier::Premium,
        thinking_level: ThinkingLevel::Disabled,
        features: VIDEO_FEATURES,
    }
}

fn response(provider: &str, model_id: &str, data: &[u8]) -> VideoGenerationResponse {
    VideoGenerationResponse {
        provider: provider.to_owned(),
        model_id: model_id.to_owned(),
        video: GeneratedVideo {
            data: data.to_vec(),
            mime_type: "video/mp4".to_owned(),
            duration_seconds: 4,
            width: 1280,
            height: 720,
        },
        usage: ModelUsage::default(),
    }
}

fn mp4() -> &'static [u8] {
    b"\0\0\0\x18ftypisom\0\0\0\0isommp42"
}
