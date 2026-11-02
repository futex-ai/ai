//! Shared error serialization and failed-attempt requirements.

use std::{error::Error, io, time::Duration};

use internal_error::InternalError;

use serde_json::json;

use crate::{
    ErrorCore, ErrorKind, ImageGenerationError, JudgmentError, ModelControl, ModelError,
    ModelEvent, TranscriptionError, VideoGenerationError,
};

#[test]
fn fallback_error_serialization_uses_the_shared_core() {
    let event = ModelEvent::Fallback {
        error: ModelError::rate_limited("provider", "model", "wait"),
        emitted_events: false,
    };

    assert_eq!(
        serde_json::to_value(event).unwrap(),
        json!({"Fallback": {
            "error": {
                "kind": "RateLimited",
                "provider": "provider",
                "model_id": "model",
                "message": "wait"
            },
            "emitted_events": false
        }})
    );
}

#[test]
fn failure_kinds_describe_fallback_eligibility() {
    for (kind, another_model_can_succeed) in [
        (ErrorKind::InvalidRequest, false),
        (ErrorKind::RejectedKey, true),
        (ErrorKind::UnknownModel, true),
        (ErrorKind::UnsupportedInput, true),
        (ErrorKind::InvalidOutput, true),
        (ErrorKind::ContentPolicy, false),
        (ErrorKind::RateLimited, true),
        (ErrorKind::Transient, true),
        (ErrorKind::Interrupted, true),
        (ErrorKind::ContextLimitExceeded, true),
        (ErrorKind::UnsupportedControl(ModelControl::Effort), true),
        (ErrorKind::TimedOut, true),
        (ErrorKind::Internal, false),
    ] {
        assert_eq!(
            kind.can_succeed_with_another_model(),
            another_model_can_succeed,
            "{kind:?}"
        );
    }
}

#[test]
fn terminal_failure_retains_history_metadata_and_original_source() {
    let earlier = ModelError::interrupted("first", "one", "closed").attempt(true);
    let mut error = ModelError::internal("last", "two", io::Error::from(io::ErrorKind::BrokenPipe));
    error.core_mut().retry_after = Some(Duration::from_secs(7));
    error.core_mut().provider_request_id = Some("request-last".to_owned());
    let error = error
        .with_attempts(vec![earlier.clone()])
        .prepend_attempts(vec![earlier.clone()]);

    assert_eq!(error.attempts(), &[earlier.clone(), earlier]);
    assert_eq!(error.core().provider, "last");
    assert_eq!(error.core().model_id, "two");
    assert_eq!(error.core().kind, ErrorKind::Internal);
    assert_eq!(error.core().retry_after, Some(Duration::from_secs(7)));
    assert_eq!(
        error.core().provider_request_id.as_deref(),
        Some("request-last")
    );
    let ModelError::Internal { source, .. } = error else {
        panic!("expected the original internal source");
    };
    assert_eq!(
        source.downcast_ref::<io::Error>().map(io::Error::kind),
        Some(io::ErrorKind::BrokenPipe)
    );
}

#[test]
fn source_failure_display_uses_the_supplied_kind_and_message() {
    let core = ErrorCore::new(
        ErrorKind::InvalidOutput,
        "provider",
        "model",
        "malformed output",
    );
    let displays = [
        (
            "model",
            ModelError::with_source(core.clone(), io::Error::from(io::ErrorKind::InvalidData))
                .to_string(),
        ),
        (
            "image_generator",
            ImageGenerationError::with_source(
                core.clone(),
                io::Error::from(io::ErrorKind::InvalidData),
            )
            .to_string(),
        ),
        (
            "video_generator",
            VideoGenerationError::with_source(
                core.clone(),
                io::Error::from(io::ErrorKind::InvalidData),
            )
            .to_string(),
        ),
        (
            "audio_transcriber",
            TranscriptionError::with_source(
                core.clone(),
                io::Error::from(io::ErrorKind::InvalidData),
            )
            .to_string(),
        ),
        (
            "judgment",
            JudgmentError::with_source(core, io::Error::from(io::ErrorKind::InvalidData))
                .to_string(),
        ),
    ];

    for (module, display) in displays {
        assert_eq!(
            display,
            format!(
                "[ai_interface/{module}] InvalidOutput for `provider` model `model`: malformed output"
            )
        );
    }
}

#[test]
fn source_failure_debug_omits_raw_source_diagnostics() {
    let secret = "auth-token-example";
    let core = ErrorCore::new(
        ErrorKind::InvalidOutput,
        "provider",
        "model",
        "safe provider diagnostics",
    );
    let model = ModelError::with_source(core.clone(), io::Error::other(secret));
    let image = ImageGenerationError::with_source(core.clone(), io::Error::other(secret));
    let video = VideoGenerationError::with_source(core.clone(), io::Error::other(secret));
    let transcription = TranscriptionError::with_source(core.clone(), io::Error::other(secret));
    let judgment = JudgmentError::with_source(core, io::Error::other(secret));
    let errors: [&dyn Error; 5] = [&model, &image, &video, &transcription, &judgment];

    for error in errors {
        let debug = format!("{error:?}");
        assert!(!debug.contains(secret), "{debug}");
        assert!(debug.contains("safe provider diagnostics"), "{debug}");
        assert!(debug.contains("InvalidOutput"), "{debug}");

        let source = error.source().unwrap();
        let source = match source.downcast_ref::<InternalError>() {
            Some(internal) => internal.source_ref(),
            None => source,
        };
        assert_eq!(
            source.downcast_ref::<io::Error>().unwrap().to_string(),
            secret
        );
    }
}
