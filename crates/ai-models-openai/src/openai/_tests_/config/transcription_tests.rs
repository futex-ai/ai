//! Configured transcription preserves multipart fields and the trimmed result.

use std::time::Duration;

use ai_interface::{AudioTranscriber, AudioTranscriptionRequest, ErrorKind};
use ai_models_core::ProviderConfig;
use json_http::JsonHttpResponse;

use crate::openai::stream_support::custom_spec;
use crate::{OpenAiAudioTranscriber, PROVIDER_ID};

use super::support::{assert_headers, buffered_client, config, multipart_fields, rotating_auth};

#[tokio::test]
async fn config_audio_uses_injected_transport_with_defaults_and_overrides() {
    for (base, timeout, expected_url, expected_timeout) in [
        (
            None,
            None,
            "https://api.openai.com/v1/audio/transcriptions",
            60,
        ),
        (
            Some("https://proxy.test/team/"),
            Some(17),
            "https://proxy.test/team/v1/audio/transcriptions",
            17,
        ),
    ] {
        let (http, requests) = buffered_client(Vec::new(), vec![transcript()]);
        let mut config = config(base)
            .with_idle_timeout(Duration::from_secs(1))
            .with_poll_interval(Duration::from_secs(1));
        config.timeouts.total_timeout = timeout.map(Duration::from_secs);
        let transcriber =
            OpenAiAudioTranscriber::from_config(http, custom_spec("custom-audio"), config);
        let response = transcriber.transcribe(&request()).await.unwrap();
        assert_eq!(response.text, "Hello world");
        let requests = requests.lock().unwrap();
        assert_eq!(requests.len(), 1);
        assert_eq!(requests[0].url, expected_url);
        assert_eq!(requests[0].timeout, Duration::from_secs(expected_timeout));
        assert_eq!(requests[0].idle_timeout, None);
        assert_headers(&requests[0], "Bearer config-key");
        let fields = multipart_fields(&requests[0]);
        assert_eq!(fields.len(), 3);
        assert_eq!(fields[0].name, "model");
        assert_eq!(fields[0].bytes, b"custom-audio");
        assert_eq!(fields[1].name, "response_format");
        assert_eq!(fields[1].bytes, b"json");
        assert_eq!(fields[2].name, "file");
        assert_eq!(fields[2].filename.as_deref(), Some("recording.webm"));
        assert_eq!(fields[2].content_type.as_deref(), Some("audio/webm"));
        assert_eq!(fields[2].bytes, [1, 2, 3]);
    }
}

#[tokio::test]
async fn config_audio_hooks_refresh_for_each_recording() {
    let (http, requests) = buffered_client(Vec::new(), vec![transcript(), transcript()]);
    let config = ProviderConfig::with_auth(PROVIDER_ID, rotating_auth())
        .with_header("authorization", "wrong")
        .with_header("X-Tenant", "tenant-value");
    let transcriber =
        OpenAiAudioTranscriber::from_config(http, custom_spec("custom-audio"), config);
    transcriber.transcribe(&request()).await.unwrap();
    transcriber.transcribe(&request()).await.unwrap();
    let requests = requests.lock().unwrap();
    assert_eq!(requests.len(), 2);
    assert_headers(&requests[0], "Bearer fresh-1");
    assert_headers(&requests[1], "Bearer fresh-2");
}

#[tokio::test]
async fn config_audio_rejects_invalid_mime_as_unsupported_input_before_dispatch() {
    let (http, requests) = buffered_client(Vec::new(), Vec::new());
    let transcriber =
        OpenAiAudioTranscriber::from_config(http, custom_spec("custom-audio"), config(None));
    let mut request = request();
    request.content_type = "invalid config-key".to_owned();
    let error = transcriber.transcribe(&request).await.unwrap_err();
    assert_eq!(error.core().kind, ErrorKind::UnsupportedInput);
    assert_eq!(error.core().provider, "openai");
    assert_eq!(error.core().model_id, "custom-audio");
    assert_eq!(
        error.core().message,
        "unsupported media type `invalid [redacted]`"
    );
    assert!(!format!("{error:?}").contains("config-key"));
    assert_eq!(requests.lock().unwrap().len(), 0);
}

fn transcript() -> JsonHttpResponse<Vec<u8>> {
    JsonHttpResponse {
        status: 200,
        headers: Default::default(),
        body: br#"{"text":"  Hello world  "}"#.to_vec(),
    }
}

fn request() -> AudioTranscriptionRequest {
    AudioTranscriptionRequest {
        audio: vec![1, 2, 3],
        filename: "recording.webm".to_owned(),
        content_type: "audio/webm".to_owned(),
    }
}

#[tokio::test]
async fn final_redirect_bodies_never_become_transcripts() {
    for status in [300, 302, 307, 308] {
        let mut response = transcript();
        response.status = status;
        let (http, _) = buffered_client(Vec::new(), vec![response]);
        let transcriber =
            OpenAiAudioTranscriber::from_config(http, custom_spec("custom-audio"), config(None));
        let result = transcriber.transcribe(&request()).await;
        assert_eq!(
            result.as_ref().err().map(|error| error.core().kind),
            Some(ErrorKind::InvalidOutput),
            "status {status}"
        );
    }
}
