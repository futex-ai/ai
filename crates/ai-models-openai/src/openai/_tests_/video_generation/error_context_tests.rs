//! Video errors retain only the relevant HTTP phase and all per-call secrets.

use std::{collections::BTreeMap, sync::Arc, time::Duration};

use ai_interface::{ErrorKind, VideoGenerationError, VideoGenerator};
use json_http::{JsonHttpAuthMock, JsonHttpResponse, StaticHeaderAuth};
use serde_json::json;
use unimock::{MockFn, Unimock, matching};

use crate::openai::{
    error_context_support::{frozen_clock, headers, unused_clock},
    video_generation::client::OpenAiVideoGenerator,
};

use super::client_tests::{advancing_runtime, request, scripted_client, stationary_runtime};

#[derive(Clone, Copy, Debug)]
enum Phase {
    Status,
    Context,
    Job,
    Poll,
    Download,
    PollTransport,
    Deadline,
}

#[tokio::test]
async fn video_failures_keep_the_failed_phase_metadata_and_decode_source() {
    for (phase, kind, id, source_kind) in [
        (
            Phase::Status,
            ErrorKind::RejectedKey,
            Some("req-submit-[redacted]"),
            "none",
        ),
        (
            Phase::Context,
            ErrorKind::ContextLimitExceeded,
            Some("req-submit-[redacted]"),
            "none",
        ),
        (
            Phase::Job,
            ErrorKind::InvalidOutput,
            Some("req-submit-[redacted]"),
            "json",
        ),
        (
            Phase::Poll,
            ErrorKind::InvalidOutput,
            Some("req-poll-[redacted]"),
            "none",
        ),
        (
            Phase::Download,
            ErrorKind::RejectedKey,
            Some("req-download-[redacted]"),
            "none",
        ),
        (Phase::PollTransport, ErrorKind::Transient, None, "http"),
        (
            Phase::Deadline,
            ErrorKind::TimedOut,
            Some("req-submit-[redacted]"),
            "none",
        ),
    ] {
        let reply = |status, id, body| {
            Ok(JsonHttpResponse {
                status,
                headers: headers(id, "Wed, 21 Oct 2015 07:28:10 GMT"),
                body,
            })
        };
        let (json_responses, byte_responses) = match phase {
            Phase::Status => (
                vec![reply(
                    401,
                    "req-submit-video-secret",
                    json!({"error":{"message":"rejected video-secret"}}),
                )],
                vec![],
            ),
            Phase::Context => (
                vec![reply(
                    400,
                    "req-submit-video-secret",
                    json!({"error":{"code":"input_too_long","message":"too many tokens"}}),
                )],
                vec![],
            ),
            Phase::Job => (
                vec![reply(
                    200,
                    "req-submit-video-secret",
                    json!({"id":"video_1","status":["video-secret"]}),
                )],
                vec![],
            ),
            Phase::Poll => (
                vec![
                    reply(
                        200,
                        "req-submit-video-secret",
                        json!({"id":"video_1","status":"queued"}),
                    ),
                    reply(
                        200,
                        "req-poll-video-secret",
                        json!({"id":"video_2","status":"completed"}),
                    ),
                ],
                vec![],
            ),
            Phase::Download => (
                vec![reply(
                    200,
                    "req-submit-video-secret",
                    json!({"id":"video_1","status":"completed"}),
                )],
                vec![Ok(JsonHttpResponse {
                    status: 403,
                    headers: headers("req-download-video-secret", "Wed, 21 Oct 2015 07:28:10 GMT"),
                    body: b"rejected video-secret".to_vec(),
                })],
            ),
            Phase::PollTransport => (
                vec![
                    reply(
                        200,
                        "req-submit-video-secret",
                        json!({"id":"video_1","status":"queued"}),
                    ),
                    Err(json_http::Error::transport("poll failed video-secret")),
                ],
                vec![],
            ),
            Phase::Deadline => (
                vec![reply(
                    200,
                    "req-submit-video-secret",
                    json!({"id":"video_1","status":"queued"}),
                )],
                vec![],
            ),
        };
        let (client, _) = scripted_client(json_responses, byte_responses);
        let runtime = if matches!(phase, Phase::Poll | Phase::PollTransport | Phase::Deadline) {
            advancing_runtime().0
        } else {
            stationary_runtime()
        };
        let clock = if id.is_some() {
            frozen_clock()
        } else {
            unused_clock()
        };
        let timeout = if matches!(phase, Phase::Deadline) {
            Duration::from_secs(2)
        } else {
            Duration::from_secs(20)
        };
        let generator = OpenAiVideoGenerator::with_auth_and_runtime(
            client,
            "sora-current",
            Arc::new(StaticHeaderAuth::bearer_token("video-secret")),
            runtime,
        )
        .with_polling(Duration::from_secs(2), timeout)
        .with_clock(clock);
        let error = generator.generate(&request()).await.unwrap_err();
        assert_eq!(error.core().kind, kind, "{phase:?}");
        assert_eq!(error.core().provider, "openai");
        assert_eq!(error.core().model_id, "sora-current");
        assert_eq!(error.core().provider_request_id.as_deref(), id, "{phase:?}");
        assert_eq!(
            error.core().retry_after,
            id.map(|_| Duration::from_secs(10)),
            "{phase:?}"
        );
        assert!(!error.to_string().contains("video-secret"), "{phase:?}");
        assert!(!format!("{error:?}").contains("video-secret"), "{phase:?}");
        let (actual_source, anchor) = match &error {
            VideoGenerationError::Internal { source, .. } => {
                let name = if source.source_ref().is::<serde_json::Error>() {
                    "json"
                } else {
                    "http"
                };
                (name, Some(source.defined_at().module_path()))
            }
            _ => ("none", None),
        };
        assert_eq!(actual_source, source_kind, "{phase:?}");
        assert_eq!(
            anchor,
            if source_kind == "none" {
                None
            } else {
                Some("ai_interface::video_generator")
            },
            "{phase:?}"
        );
    }
}

#[tokio::test]
async fn rotating_video_auth_keeps_submission_and_poll_secrets_out_of_failures() {
    let auth = Unimock::new(JsonHttpAuthMock::apply_headers.stub(|calls| {
        calls
            .call(matching!(_))
            .answers(&|_, headers: &mut BTreeMap<String, String>| {
                headers.insert("Authorization".into(), "Bearer submit-secret".into());
                Ok(())
            })
            .once()
            .then()
            .answers(&|_, headers: &mut BTreeMap<String, String>| {
                headers.insert("Authorization".into(), "Bearer poll-secret".into());
                Ok(())
            })
            .once();
    }));
    let (client, requests) = scripted_client(
        vec![
            Ok(JsonHttpResponse {
                status: 200,
                headers: Default::default(),
                body: json!({"id":"video_1","status":"queued"}),
            }),
            Ok(JsonHttpResponse {
                status: 403,
                headers: headers("req-poll-secret", "7"),
                body: json!({"error":{"message":"rejected submit-secret poll-secret"}}),
            }),
        ],
        vec![],
    );
    let (runtime, _) = advancing_runtime();
    let generator = OpenAiVideoGenerator::with_auth_and_runtime(
        client,
        "sora-current",
        Arc::new(auth),
        runtime,
    )
    .with_clock(unused_clock());
    let error = generator.generate(&request()).await.unwrap_err();
    assert_eq!(error.core().kind, ErrorKind::RejectedKey);
    assert_eq!(error.core().message, "rejected [redacted] [redacted]");
    assert_eq!(
        error.core().provider_request_id.as_deref(),
        Some("req-[redacted]")
    );
    assert_eq!(error.core().retry_after, Some(Duration::from_secs(7)));
    assert!(!format!("{error:?}").contains("submit-secret"));
    assert!(!format!("{error:?}").contains("poll-secret"));
    let requests = requests.lock().unwrap();
    assert_eq!(requests.len(), 2);
    assert_eq!(
        requests[0].headers.get("Authorization").map(String::as_str),
        Some("Bearer submit-secret")
    );
    assert_eq!(
        requests[1].headers.get("Authorization").map(String::as_str),
        Some("Bearer poll-secret")
    );
}
