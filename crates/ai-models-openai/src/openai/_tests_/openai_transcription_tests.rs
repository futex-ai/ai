//! Tests for OpenAI audio transcription configuration.

use crate::openai::stream_support::error_context;

use std::time::Duration;

use reqwest::StatusCode;

use super::{OpenAiAudioTranscriber, classify_status};

#[test]
fn transcriber_uses_default_request_timeout() {
    let transcriber = OpenAiAudioTranscriber::new("gpt-4o-mini-transcribe", "sk-openai");

    assert_eq!(transcriber.timeout, Duration::from_secs(60));
}

#[test]
fn retryable_http_statuses_are_transient_provider_failures() {
    let statuses = [
        StatusCode::REQUEST_TIMEOUT,
        StatusCode::CONFLICT,
        StatusCode::from_u16(425).expect("425 should be a valid HTTP status"),
    ];

    for status in statuses {
        assert!(
            matches!(classify_status(&error_context("gpt-4o-mini-transcribe"), status, &Default::default(), "retry later".to_owned()), ref error if error.core().kind == ai_interface::ErrorKind::Transient)
        );
    }
}
