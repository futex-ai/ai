//! Shared provider settings reach Gemini image requests and local errors.

use std::{collections::BTreeMap, sync::Arc, time::Duration};

use ai_interface::{
    ErrorKind, ImageGenerationError, ImageGenerationInputImage, ImageGenerationRequest,
    ImageGenerator,
};
use ai_models_core::ProviderConfig;
use json_http::{StaticHeaderAuth, TransportBackedJsonHttpClient};
use unimock::Unimock;

use crate::{GEMINI_3_1_FLASH_IMAGE, GoogleImageGenerator, PROVIDER_ID};

use super::client_tests::recording_http_client;

#[tokio::test]
async fn config_image_uses_base_headers_and_only_its_total_timeout() {
    for (base, total, endpoint, expected_key) in [
        (
            None,
            None,
            "https://generativelanguage.googleapis.com/v1/models/gemini-3.1-flash-image:generateContent",
            "config-key",
        ),
        (
            Some("https://google.test/proxy"),
            Some(31),
            "https://google.test/proxy/v1/models/gemini-3.1-flash-image:generateContent",
            "hook-key",
        ),
        (
            Some("https://google.test/proxy/"),
            Some(31),
            "https://google.test/proxy/v1/models/gemini-3.1-flash-image:generateContent",
            "hook-key",
        ),
    ] {
        let (client, requests) = recording_http_client();
        let config = if base.is_some() {
            ProviderConfig::with_auth(
                PROVIDER_ID,
                Arc::new(StaticHeaderAuth::new(BTreeMap::from([(
                    "x-goog-api-key".into(),
                    "hook-key".into(),
                )]))),
            )
        } else {
            ProviderConfig::new(PROVIDER_ID, "config-key")
        };
        let mut config = config
            .with_header("X-Route", "proxy")
            .with_header("X-GOOG-API-KEY", "extra-key")
            .with_idle_timeout(Duration::from_secs(17))
            .with_poll_interval(Duration::from_secs(3));
        config.base_url = base.map(str::to_owned);
        config.timeouts.total_timeout = total.map(Duration::from_secs);
        let generator = GoogleImageGenerator::from_config(client, GEMINI_3_1_FLASH_IMAGE, config);

        let response = generator
            .generate(&ImageGenerationRequest {
                prompt: "Draw a blue circle".into(),
                ..Default::default()
            })
            .await
            .unwrap();

        assert_eq!(response.model_id, "gemini-3.1-flash-image");
        let requests = requests.lock().unwrap();
        assert_eq!(requests.len(), 1);
        assert_eq!(requests[0].url, endpoint);
        assert_eq!(
            requests[0].headers.get("X-Route").map(String::as_str),
            Some("proxy")
        );
        let keys: Vec<_> = requests[0]
            .headers
            .iter()
            .filter(|(name, _)| name.eq_ignore_ascii_case("x-goog-api-key"))
            .map(|(_, value)| value.as_str())
            .collect();
        assert_eq!(keys, vec![expected_key]);
        assert_eq!(
            requests[0].timeout,
            Duration::from_secs(total.unwrap_or(120))
        );
        assert_eq!(requests[0].idle_timeout, None);
    }
}

#[tokio::test]
async fn config_image_captures_explicit_key_before_local_validation() {
    let generator = GoogleImageGenerator::from_config(
        Arc::new(TransportBackedJsonHttpClient::new(Arc::new(Unimock::new(
            (),
        )))),
        GEMINI_3_1_FLASH_IMAGE,
        ProviderConfig::new(PROVIDER_ID, "config-secret"),
    );
    let error = generator
        .generate(&ImageGenerationRequest {
            prompt: "Draw a bird".into(),
            input_images: vec![ImageGenerationInputImage {
                data: vec![1],
                mime_type: "config-secret".into(),
            }],
            ..Default::default()
        })
        .await
        .unwrap_err();

    assert_eq!(error.core().kind, ErrorKind::UnsupportedInput);
    assert!(!error.to_string().contains("config-secret"));
    assert!(!format!("{error:?}").contains("config-secret"));
    let content_type = match error {
        ImageGenerationError::UnsupportedMediaType { content_type, .. } => Some(content_type),
        _ => None,
    };
    assert_eq!(content_type.as_deref(), Some("[redacted]"));
}
