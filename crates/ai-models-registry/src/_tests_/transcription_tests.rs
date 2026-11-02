//! Registry transcription uses the injected client and the supplied custom spec.

use std::time::Duration;

use ai_interface::AudioTranscriptionRequest;
use serde_json::json;

use crate::{ProviderConfig, Registry};

use super::{media_support::bytes_client, support::spec};

#[tokio::test]
async fn transcription_factory_forwards_custom_model_and_provider_settings() {
    let (http, requests) = bytes_client(json!({"text":"A bird sings"}));
    let spec = spec("openai");
    let registry = Registry::new(http).with_provider(
        ProviderConfig::new(spec.provider.clone(), "registry-key")
            .with_base_url("https://proxy.test/team/")
            .with_header("X-Tenant", "tenant")
            .with_total_timeout(Duration::from_secs(31)),
    );
    let built = registry.transcription(&spec);
    assert!(built.is_ok());

    let response = built
        .unwrap()
        .transcribe(&AudioTranscriptionRequest {
            audio: vec![1],
            filename: "bird.wav".into(),
            content_type: "audio/wav".into(),
        })
        .await
        .unwrap();

    assert_eq!(response.text, "A bird sings");
    let requests = requests.lock().unwrap();
    assert_eq!(requests.len(), 1);
    assert_eq!(
        requests[0].url,
        "https://proxy.test/team/v1/audio/transcriptions"
    );
    assert_eq!(
        requests[0].headers.get("Authorization").map(String::as_str),
        Some("Bearer registry-key")
    );
    assert_eq!(
        requests[0].headers.get("X-Tenant").map(String::as_str),
        Some("tenant")
    );
    assert_eq!(requests[0].timeout, Duration::from_secs(31));
    let fields = match requests[0].body.as_ref().unwrap() {
        json_http::JsonHttpBody::Multipart(body) => body.fields.as_slice(),
        _ => panic!("transcription request must be multipart"),
    };
    assert_eq!(
        fields
            .iter()
            .find(|field| field.name == "model")
            .unwrap()
            .bytes,
        b"unlisted-model"
    );
}
