//! Configured image generation and multipart editing requests.

use std::time::Duration;

use ai_interface::{ImageGenerationInputImage, ImageGenerationRequest, ImageGenerator};
use ai_models_core::ProviderConfig;
use serde_json::json;

use crate::{GPT_IMAGE_2, OpenAiImageGenerator, PROVIDER_ID};

use super::support::{
    assert_headers, buffered_client, config, json_response, multipart_fields, rotating_auth,
};

#[tokio::test]
async fn config_images_apply_defaults_and_overrides_to_generation_and_edits() {
    for (base, timeout, prefix, expected_timeout) in [
        (None, None, "https://api.openai.com", 120),
        (
            Some("https://proxy.test/team/"),
            Some(27),
            "https://proxy.test/team",
            27,
        ),
    ] {
        let (http, requests) = buffered_client(vec![image(), image()], Vec::new());
        let mut config = config(base)
            .with_idle_timeout(Duration::from_secs(1))
            .with_poll_interval(Duration::from_secs(1));
        config.timeouts.total_timeout = timeout.map(Duration::from_secs);
        let generator = OpenAiImageGenerator::from_config(http, GPT_IMAGE_2, config);
        generator.generate(&request(false)).await.unwrap();
        generator.generate(&request(true)).await.unwrap();

        let requests = requests.lock().unwrap();
        assert_eq!(requests.len(), 2);
        assert_eq!(requests[0].url, format!("{prefix}/v1/images/generations"));
        assert_eq!(requests[1].url, format!("{prefix}/v1/images/edits"));
        for request in requests.iter() {
            assert_eq!(request.timeout, Duration::from_secs(expected_timeout));
            assert_eq!(request.idle_timeout, None);
            assert_headers(request, "Bearer config-key");
        }
        assert_eq!(multipart_fields(&requests[1])[0].bytes, b"gpt-image-2");
    }
}

#[tokio::test]
async fn config_image_hooks_run_for_json_and_multipart_requests() {
    let (http, requests) = buffered_client(vec![image(), image()], Vec::new());
    let config = ProviderConfig::with_auth(PROVIDER_ID, rotating_auth())
        .with_header("authorization", "wrong")
        .with_header("X-Tenant", "tenant-value");
    let generator = OpenAiImageGenerator::from_config(http, GPT_IMAGE_2, config);
    generator.generate(&request(false)).await.unwrap();
    generator.generate(&request(true)).await.unwrap();
    let requests = requests.lock().unwrap();
    assert_eq!(requests.len(), 2);
    assert_headers(&requests[0], "Bearer fresh-1");
    assert_headers(&requests[1], "Bearer fresh-2");
}

fn request(edit: bool) -> ImageGenerationRequest {
    ImageGenerationRequest {
        prompt: "A blue circle".to_owned(),
        input_images: if edit {
            vec![ImageGenerationInputImage {
                data: vec![1, 2],
                mime_type: "image/png".to_owned(),
            }]
        } else {
            Vec::new()
        },
        ..Default::default()
    }
}

fn image() -> json_http::JsonHttpResponse<serde_json::Value> {
    json_response(json!({"data":[{"b64_json":"iVBORw0KGgo="}], "output_format":"png"}))
}
