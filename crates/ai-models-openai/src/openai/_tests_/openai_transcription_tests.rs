//! Tests for OpenAI audio transcription configuration.

use std::time::Duration;

use super::OpenAiAudioTranscriber;

#[test]
fn transcriber_uses_default_request_timeout() {
    let transcriber = OpenAiAudioTranscriber::new("gpt-4o-mini-transcribe", "sk-openai");

    assert_eq!(transcriber.timeout, Duration::from_secs(60));
}
