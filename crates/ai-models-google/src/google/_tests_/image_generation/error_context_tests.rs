//! Google image errors keep the original response facts and inspectable decode source.

use std::{sync::Arc, time::Duration};

use ai_interface::{
    ErrorKind, ImageGenerationError, ImageGenerationInputImage, ImageGenerationRequest,
    ImageGenerator,
};
use json_http::{JsonHttpResponse, JsonHttpTransportMock, TransportBackedJsonHttpClient};
use serde_json::json;
use unimock::{MockFn, Unimock, matching};

use crate::google::{
    error_context_support::{frozen_clock, headers, unused_clock},
    image_generation::client::GoogleImageGenerator,
};

#[tokio::test]
async fn embedded_image_errors_keep_headers_and_context_precedes_policy() {
    for (code, status, expected, policy, decode_error) in [
        (
            400,
            "context_length_exceeded",
            ErrorKind::ContextLimitExceeded,
            true,
            false,
        ),
        (
            403,
            "PERMISSION_DENIED",
            ErrorKind::RejectedKey,
            false,
            false,
        ),
        (
            429,
            "RESOURCE_EXHAUSTED",
            ErrorKind::RateLimited,
            false,
            true,
        ),
    ] {
        let mut body = json!({"error":{"code":code,"status":status,"message":"rejected google-secret"},"promptFeedback":if policy {json!({"blockReason":"SAFETY","blockReasonMessage":"policy google-secret"})} else {json!({})}});
        if decode_error {
            body["candidates"] = json!(["google-secret"]);
        }
        let transport = Unimock::new(
            JsonHttpTransportMock::execute
                .some_call(matching!(_))
                .returns(Ok(JsonHttpResponse {
                    status: 200,
                    headers: headers("req-google-secret", "7"),
                    body,
                })),
        );
        let generator = GoogleImageGenerator::new(
            Arc::new(TransportBackedJsonHttpClient::new(Arc::new(transport))),
            "gemini-image-current",
            "google-secret",
        )
        .with_clock(unused_clock());
        let error = generator.generate(&request()).await.unwrap_err();
        assert_eq!(error.core().kind, expected, "{code}");
        assert_eq!(
            error.core().provider_request_id.as_deref(),
            Some("req-[redacted]")
        );
        assert_eq!(error.core().retry_after, Some(Duration::from_secs(7)));
        assert!(!error.to_string().contains("google-secret"));
        assert!(!format!("{error:?}").contains("google-secret"));
        let source_facts = match &error {
            ImageGenerationError::Internal { source, .. } => Some((
                source.source_ref().is::<serde_json::Error>(),
                source.defined_at().module_path(),
            )),
            _ => None,
        };
        assert_eq!(
            source_facts,
            if decode_error {
                Some((true, "ai_interface::image_generator"))
            } else {
                None
            }
        );
    }
}

#[tokio::test]
async fn image_status_and_late_output_errors_keep_metadata_and_original_sources() {
    for (case, status, body, kind, source_kind, source_module) in [
        (
            "status",
            401,
            json!({"error":{"message":"rejected google-secret"}}),
            ErrorKind::RejectedKey,
            "none",
            None,
        ),
        (
            "json",
            200,
            json!({"candidates":["google-secret"]}),
            ErrorKind::InvalidOutput,
            "json",
            Some("ai_interface::image_generator"),
        ),
        (
            "base64",
            200,
            json!({"candidates":[{"content":{"parts":[{"inlineData":{"mimeType":"image/png","data":"not base64"}}]}}]}),
            ErrorKind::InvalidOutput,
            "base64",
            Some("ai_interface::image_generator"),
        ),
        (
            "missing data",
            200,
            json!({"candidates":[{"content":{"parts":[{"inlineData":{"mimeType":"image/png"}}]}}]}),
            ErrorKind::InvalidOutput,
            "none",
            None,
        ),
        (
            "missing MIME",
            200,
            json!({"candidates":[{"content":{"parts":[{"inlineData":{"data":"AQID"}}]}}]}),
            ErrorKind::InvalidOutput,
            "none",
            None,
        ),
        (
            "policy",
            200,
            json!({"promptFeedback":{"blockReason":"SAFETY","blockReasonMessage":"blocked google-secret"}}),
            ErrorKind::ContentPolicy,
            "none",
            None,
        ),
        (
            "empty",
            200,
            json!({"candidates":[]}),
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
                    headers: headers("req-google-secret", "Wed, 21 Oct 2015 07:28:10 GMT"),
                    body,
                })),
        );
        let generator = GoogleImageGenerator::new(
            Arc::new(TransportBackedJsonHttpClient::new(Arc::new(transport))),
            "gemini-image-current",
            "google-secret",
        )
        .with_clock(frozen_clock());
        let error = generator.generate(&request()).await.unwrap_err();
        assert_eq!(error.core().kind, kind, "{case}");
        assert_eq!(error.core().provider, "google");
        assert_eq!(error.core().model_id, "gemini-image-current");
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
        assert!(!error.to_string().contains("google-secret"), "{case}");
        assert!(!format!("{error:?}").contains("google-secret"), "{case}");
        let (actual_source, anchor, echo) = match &error {
            ImageGenerationError::Internal { source, .. } => (
                if source.source_ref().is::<serde_json::Error>() {
                    "json"
                } else {
                    "base64"
                },
                Some(source.defined_at().module_path()),
                source.source_ref().to_string().contains("google-secret"),
            ),
            _ => ("none", None, false),
        };
        assert_eq!(actual_source, source_kind, "{case}");
        assert_eq!(anchor, source_module, "{case}");
        assert_eq!(echo, source_kind == "json", "{case}");
    }
}

#[tokio::test]
async fn local_image_media_details_hide_the_explicit_key_without_transport() {
    let generator = GoogleImageGenerator::new(
        Arc::new(TransportBackedJsonHttpClient::new(Arc::new(Unimock::new(
            (),
        )))),
        "gemini-image-current",
        "google-secret",
    )
    .with_clock(unused_clock());
    let error = generator
        .generate(&ImageGenerationRequest {
            input_images: vec![ImageGenerationInputImage {
                data: vec![1],
                mime_type: "google-secret".into(),
            }],
            ..request()
        })
        .await
        .unwrap_err();
    assert_eq!(error.core().kind, ErrorKind::UnsupportedInput);
    assert_eq!(error.core().provider, "google");
    assert_eq!(error.core().model_id, "gemini-image-current");
    assert_eq!(error.core().provider_request_id, None);
    assert!(!error.to_string().contains("google-secret"));
    assert!(!format!("{error:?}").contains("google-secret"));
    assert!(
        matches!(error,ImageGenerationError::UnsupportedMediaType {content_type,..} if content_type == "[redacted]")
    );
}

fn request() -> ImageGenerationRequest {
    ImageGenerationRequest {
        prompt: "draw a bird".into(),
        ..Default::default()
    }
}
