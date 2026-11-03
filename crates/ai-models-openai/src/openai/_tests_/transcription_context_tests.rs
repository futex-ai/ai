//! In-memory reqwest responses exercise the unchanged audio response boundary.

use std::{error::Error, time::Duration};

use ai_interface::{ErrorKind, SafeErrorSource, TranscriptionError};
use reqwest::{IntoUrl, Response};
use serde_json::json;

use crate::openai::error_context_support::{frozen_clock, headers, unused_clock};

use super::{OpenAiAudioTranscriber, request_failure};

#[tokio::test]
async fn audio_status_and_decode_failures_keep_headers_identity_and_reqwest_sources() {
    for (case, status, body, kind, source_expected, empty) in [
        (
            "status",
            401,
            "rejected audio-secret".to_owned(),
            ErrorKind::RejectedKey,
            false,
            false,
        ),
        (
            "context",
            400,
            json!({"error":{"code":"input_too_long","message":"too much audio"}}).to_string(),
            ErrorKind::ContextLimitExceeded,
            false,
            false,
        ),
        (
            "decode",
            200,
            "not JSON audio-secret".to_owned(),
            ErrorKind::InvalidOutput,
            true,
            false,
        ),
        (
            "empty",
            200,
            json!({"text":"  "}).to_string(),
            ErrorKind::InvalidOutput,
            false,
            true,
        ),
    ] {
        let transcriber =
            OpenAiAudioTranscriber::new("audio-current", "audio-secret").with_clock(frozen_clock());
        let mut response = http::Response::builder().status(status).body(body).unwrap();
        *response.headers_mut() = headers("req-audio-secret", "Wed, 21 Oct 2015 07:28:10 GMT");
        let error = transcriber
            .read_response(&transcriber.context(), Response::from(response))
            .await
            .unwrap_err();
        assert_eq!(error.core().kind, kind, "{case}");
        assert_eq!(error.core().provider, "openai");
        assert_eq!(error.core().model_id, "audio-current");
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
        assert!(!error.to_string().contains("audio-secret"), "{case}");
        assert!(!format!("{error:?}").contains("audio-secret"), "{case}");
        assert_eq!(
            error
                .source()
                .and_then(|source| source
                    .downcast_ref::<SafeErrorSource>()
                    .unwrap()
                    .raw_cause()
                    .downcast_ref::<reqwest::Error>())
                .is_some(),
            source_expected,
            "{case}"
        );
        assert_eq!(
            matches!(error, TranscriptionError::EmptyTranscript { .. }),
            empty,
            "{case}"
        );
    }
}

#[test]
fn audio_request_diagnostics_omit_signed_urls_and_keep_the_original_source() {
    let transcriber =
        OpenAiAudioTranscriber::new("audio-current", "audio-secret").with_clock(unused_clock());
    let signed = "https://audio.test/input?token=url-secret";
    let status_source = Response::from(http::Response::builder().status(503).body("").unwrap())
        .error_for_status()
        .unwrap_err();
    for (cause, kind) in [
        (invalid_url("://invalid"), ErrorKind::Internal),
        (status_source, ErrorKind::Transient),
    ] {
        let error = request_failure(
            &transcriber.context(),
            cause.with_url(signed.parse().unwrap()),
        );
        assert_eq!(error.core().kind, kind);
        assert!(!error.to_string().contains("url-secret"));
        assert!(!format!("{error:?}").contains("url-secret"));
        let source = error
            .source()
            .unwrap()
            .downcast_ref::<SafeErrorSource>()
            .unwrap()
            .raw_cause()
            .downcast_ref::<reqwest::Error>()
            .unwrap();
        assert_eq!(source.url().unwrap().as_str(), signed);
    }
}

fn invalid_url(url: impl IntoUrl) -> reqwest::Error {
    url.into_url().unwrap_err()
}
