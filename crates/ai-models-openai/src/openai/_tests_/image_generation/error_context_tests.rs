//! Image status and output failures retain safe metadata and typed sources.

use std::{sync::Arc, time::Duration};

use ai_interface::{
    ErrorKind, ImageGenerationError, ImageGenerationInputImage, ImageGenerationRequest,
    ImageGenerator,
};
use json_http::{JsonHttpResponse, JsonHttpTransportMock, TransportBackedJsonHttpClient};
use serde_json::json;
use unimock::{MockFn, Unimock, matching};

use crate::openai::{
    error_context_support::{frozen_clock, headers, unused_clock},
    image_generation::client::OpenAiImageGenerator,
};

#[tokio::test]
async fn image_failures_keep_response_metadata_and_original_decode_sources() {
    for (case, status, body, kind, source_kind, source_module) in [
        (
            "status",
            401,
            json!({"error":{"message":"rejected image-secret"}}),
            ErrorKind::RejectedKey,
            "none",
            None,
        ),
        (
            "context",
            400,
            json!({"error":{"code":"context_length_exceeded","message":"too many tokens"}}),
            ErrorKind::ContextLimitExceeded,
            "none",
            None,
        ),
        (
            "json",
            200,
            json!({"data":"image-secret"}),
            ErrorKind::InvalidOutput,
            "json",
            Some("ai_interface::image_generator"),
        ),
        (
            "base64",
            200,
            json!({"data":[{"b64_json":"not base64"}]}),
            ErrorKind::InvalidOutput,
            "base64",
            Some("ai_interface::image_generator"),
        ),
        (
            "format",
            200,
            json!({"data":[{"b64_json":"AQID"}]}),
            ErrorKind::InvalidOutput,
            "format",
            Some("ai_interface::image_generator"),
        ),
        (
            "empty",
            200,
            json!({"data":[]}),
            ErrorKind::InvalidOutput,
            "none",
            None,
        ),
    ] {
        let transport = Unimock::new(
            JsonHttpTransportMock::execute
                .some_call(matching!(_))
                .returns(Ok(JsonHttpResponse {
                    status,
                    headers: headers("req-image-secret", "Wed, 21 Oct 2015 07:28:10 GMT"),
                    body,
                })),
        );
        let generator = OpenAiImageGenerator::new(
            Arc::new(TransportBackedJsonHttpClient::new(Arc::new(transport))),
            "gpt-image-current",
            "image-secret",
        )
        .with_clock(frozen_clock());
        let error = generator
            .generate(&ImageGenerationRequest {
                prompt: "draw a bird".into(),
                ..Default::default()
            })
            .await
            .unwrap_err();
        assert_eq!(error.core().kind, kind, "{case}");
        assert_eq!(error.core().provider, "openai");
        assert_eq!(error.core().model_id, "gpt-image-current");
        assert_eq!(
            error.core().provider_request_id.as_deref(),
            Some("req-[redacted]"),
            "{case}"
        );
        assert_eq!(
            error.core().retry_after,
            Some(Duration::from_secs(10)),
            "{case}"
        );
        assert!(!error.to_string().contains("image-secret"), "{case}");
        assert!(!format!("{error:?}").contains("image-secret"), "{case}");
        let (actual_source, anchor) = match &error {
            ImageGenerationError::Internal { source, .. } => {
                let name = if source.source_ref().is::<serde_json::Error>() {
                    "json"
                } else if source.source_ref().is::<base64::DecodeError>() {
                    "base64"
                } else {
                    "format"
                };
                (name, Some(source.defined_at().module_path()))
            }
            _ => ("none", None),
        };
        assert_eq!(actual_source, source_kind, "{case}");
        assert_eq!(anchor, source_module, "{case}");
        let original_source_echo = match &error {
            ImageGenerationError::Internal { source, .. } => {
                source.source_ref().to_string().contains("image-secret")
            }
            _ => false,
        };
        assert_eq!(original_source_echo, source_kind == "json", "{case}");
    }
}

#[tokio::test]
async fn image_local_media_details_redact_an_explicit_key_without_dispatch() {
    let generator = OpenAiImageGenerator::new(
        Arc::new(TransportBackedJsonHttpClient::new(Arc::new(Unimock::new(
            (),
        )))),
        "gpt-image-current",
        "image-secret",
    )
    .with_clock(unused_clock());
    let error = generator
        .generate(&ImageGenerationRequest {
            prompt: "draw a bird".into(),
            input_images: vec![ImageGenerationInputImage {
                data: vec![1],
                mime_type: "image-secret".into(),
            }],
            ..Default::default()
        })
        .await
        .unwrap_err();
    assert_eq!(error.core().kind, ErrorKind::UnsupportedInput);
    assert_eq!(error.core().provider, "openai");
    assert_eq!(error.core().model_id, "gpt-image-current");
    assert_eq!(error.core().provider_request_id, None);
    assert!(!error.to_string().contains("image-secret"));
    assert!(!format!("{error:?}").contains("image-secret"));
    assert!(
        matches!(error,ImageGenerationError::UnsupportedMediaType {content_type,..} if content_type == "[redacted]")
    );
}
