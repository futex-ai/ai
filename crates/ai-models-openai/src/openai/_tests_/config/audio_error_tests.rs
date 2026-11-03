//! Injected audio responses retain the existing parser and error facts.

use std::{error::Error, sync::Arc, time::Duration};

use ai_interface::{
    AudioTranscriber, AudioTranscriptionRequest, ErrorKind, SafeErrorSource, TranscriptionError,
};
use json_http::{JsonHttpResponse, JsonHttpTransportMock, TransportBackedJsonHttpClient};
use unimock::{MockFn, Unimock, matching};

use crate::OpenAiAudioTranscriber;
use crate::openai::{
    error_context_support::{headers, unused_clock},
    stream_support::custom_spec,
};

use super::support::{buffered_client, config};

#[tokio::test]
async fn config_audio_response_errors_preserve_kinds_headers_and_decode_sources() {
    for (case, status, body, kind, decode_source, empty) in [
        (
            "status",
            401,
            b"rejected config-key".to_vec(),
            ErrorKind::RejectedKey,
            false,
            false,
        ),
        (
            "context",
            400,
            br#"{"error":{"code":"input_too_long"}}"#.to_vec(),
            ErrorKind::ContextLimitExceeded,
            false,
            false,
        ),
        (
            "json",
            200,
            b"invalid JSON config-key".to_vec(),
            ErrorKind::InvalidOutput,
            true,
            false,
        ),
        (
            "utf8",
            200,
            vec![255, 254],
            ErrorKind::InvalidOutput,
            true,
            false,
        ),
        (
            "empty",
            200,
            br#"{"text":"  "}"#.to_vec(),
            ErrorKind::InvalidOutput,
            false,
            true,
        ),
    ] {
        let (http, requests) = buffered_client(
            Vec::new(),
            vec![JsonHttpResponse {
                status,
                headers: headers("req-config-key", "5"),
                body,
            }],
        );
        let transcriber =
            OpenAiAudioTranscriber::from_config(http, custom_spec("custom-audio"), config(None))
                .with_clock(unused_clock());
        let error = transcriber
            .transcribe(&AudioTranscriptionRequest {
                audio: vec![1],
                filename: "audio.webm".to_owned(),
                content_type: "audio/webm".to_owned(),
            })
            .await
            .unwrap_err();
        assert_eq!(error.core().kind, kind, "{case}");
        assert_eq!(error.core().provider, "openai", "{case}");
        assert_eq!(error.core().model_id, "custom-audio", "{case}");
        assert_eq!(
            error.core().provider_request_id.as_deref(),
            Some("req-[redacted]"),
            "{case}"
        );
        assert_eq!(
            error.core().retry_after,
            Some(Duration::from_secs(5)),
            "{case}"
        );
        assert_eq!(
            error
                .source()
                .and_then(|source| source
                    .downcast_ref::<SafeErrorSource>()
                    .unwrap()
                    .raw_cause()
                    .downcast_ref::<reqwest::Error>())
                .is_some(),
            decode_source,
            "{case}"
        );
        assert_eq!(
            matches!(error, TranscriptionError::EmptyTranscript { .. }),
            empty,
            "{case}"
        );
        assert!(!error.to_string().contains("config-key"), "{case}");
        assert!(!format!("{error:?}").contains("config-key"), "{case}");
        assert_eq!(requests.lock().unwrap().len(), 1, "{case}");
    }
}

#[tokio::test]
async fn config_audio_byte_read_failure_retains_response_metadata() {
    let transport = Unimock::new(
        JsonHttpTransportMock::execute_bytes
            .some_call(matching!(_))
            .returns(Err(json_http::Error::transport("read failed config-key")
                .with_response_headers(headers("req-config-key", "5")))),
    );
    let transcriber = OpenAiAudioTranscriber::from_config(
        Arc::new(TransportBackedJsonHttpClient::new(Arc::new(transport))),
        custom_spec("custom-audio"),
        config(None),
    )
    .with_clock(unused_clock());
    let error = transcriber
        .transcribe(&AudioTranscriptionRequest {
            audio: vec![1],
            filename: "audio.webm".to_owned(),
            content_type: "audio/webm".to_owned(),
        })
        .await
        .unwrap_err();
    assert_eq!(error.core().kind, ErrorKind::Transient);
    assert_eq!(error.core().provider, "openai");
    assert_eq!(error.core().model_id, "custom-audio");
    assert_eq!(
        error.core().provider_request_id.as_deref(),
        Some("req-[redacted]")
    );
    assert_eq!(error.core().retry_after, Some(Duration::from_secs(5)));
    assert_eq!(
        error.core().message,
        "[json_http/error] transport error: read failed [redacted]"
    );
    assert!(matches!(
        error
            .source()
            .unwrap()
            .downcast_ref::<SafeErrorSource>()
            .unwrap()
            .raw_cause()
            .downcast_ref::<json_http::Error>()
            .unwrap()
            .cause(),
        json_http::Error::Transport { .. }
    ));
    assert!(!error.to_string().contains("config-key"));
    assert!(!format!("{error:?}").contains("config-key"));
}
