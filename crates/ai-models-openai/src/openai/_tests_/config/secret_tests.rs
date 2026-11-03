//! Configured credentials are redacted before local media validation.

use ai_interface::{ErrorKind, ImageGenerationInputImage, ImageGenerationRequest, ImageGenerator};

use crate::{GPT_IMAGE_2, OpenAiImageGenerator};

use super::support::{buffered_client, config};

#[tokio::test]
async fn config_media_validation_redacts_keys_before_auth_runs() {
    let (http, _) = buffered_client(Vec::new(), Vec::new());
    let image = OpenAiImageGenerator::from_config(http, GPT_IMAGE_2, config(None));
    let error = image
        .generate(&ImageGenerationRequest {
            prompt: "Edit this".to_owned(),
            input_images: vec![ImageGenerationInputImage {
                data: vec![1],
                mime_type: "config-key".to_owned(),
            }],
            ..Default::default()
        })
        .await
        .unwrap_err();
    assert_eq!(error.core().kind, ErrorKind::UnsupportedInput);
    assert_eq!(error.core().message, "unsupported media type `[redacted]`");
    assert!(!error.to_string().contains("config-key"));
    assert!(!format!("{error:?}").contains("config-key"));
}
