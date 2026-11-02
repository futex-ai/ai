//! Google operation and HTTP phase errors preserve their own response metadata.

use std::{collections::BTreeMap, sync::Arc, time::Duration};

use ai_interface::{ErrorKind, VideoGenerationError, VideoGenerator};
use json_http::{JsonHttpAuthMock, JsonHttpResponse, StaticHeaderAuth};
use serde_json::json;
use unimock::{MockFn, Unimock, matching};

use crate::google::{
    error_context_support::{frozen_clock, headers, unused_clock},
    video_generation::client::GoogleVideoGenerator,
};

use super::client_tests::{advancing_runtime, request, scripted_client, stationary_runtime};

const OPERATION: &str = "models/veo-current/operations/op_1";

#[derive(Clone, Copy, Debug)]
enum Phase {
    Status,
    Json,
    Name,
    Uri,
    Poll,
    Download,
    PollTransport,
    Deadline,
}

#[tokio::test]
async fn video_output_and_phase_errors_keep_metadata_sources_and_origin_guards() {
    for (phase, kind, id, source_kind) in [
        (
            Phase::Status,
            ErrorKind::RejectedKey,
            Some("req-submit-[redacted]"),
            "none",
        ),
        (
            Phase::Json,
            ErrorKind::InvalidOutput,
            Some("req-submit-[redacted]"),
            "json",
        ),
        (
            Phase::Name,
            ErrorKind::InvalidOutput,
            Some("req-submit-[redacted]"),
            "none",
        ),
        (
            Phase::Uri,
            ErrorKind::InvalidOutput,
            Some("req-submit-[redacted]"),
            "none",
        ),
        (
            Phase::Poll,
            ErrorKind::InvalidOutput,
            Some("req-poll-[redacted]"),
            "none",
        ),
        (
            Phase::Download,
            ErrorKind::InvalidOutput,
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
        let complete = |uri| json!({"name":OPERATION,"done":true,"response":{"generateVideoResponse":{"generatedSamples":[{"video":{"uri":uri}}]}}});
        let (json_responses, bytes) = match phase {
            Phase::Status => (
                vec![reply(
                    401,
                    "req-submit-google-secret",
                    json!({"error":{"message":"rejected google-secret"}}),
                )],
                vec![],
            ),
            Phase::Json => (
                vec![reply(
                    200,
                    "req-submit-google-secret",
                    json!({"name":["google-secret"]}),
                )],
                vec![],
            ),
            Phase::Name => (
                vec![reply(
                    200,
                    "req-submit-google-secret",
                    json!({"name":"../operation"}),
                )],
                vec![],
            ),
            Phase::Uri => (
                vec![reply(
                    200,
                    "req-submit-google-secret",
                    complete("https://attacker.test/video.mp4"),
                )],
                vec![],
            ),
            Phase::Poll => (
                vec![
                    reply(200, "req-submit-google-secret", json!({"name":OPERATION})),
                    reply(
                        200,
                        "req-poll-google-secret",
                        json!({"name":"models/veo-current/operations/op_2","done":true}),
                    ),
                ],
                vec![],
            ),
            Phase::Download => (
                vec![reply(
                    200,
                    "req-submit-google-secret",
                    complete("https://google.test/v1beta/files/1"),
                )],
                vec![Ok(JsonHttpResponse {
                    status: 200,
                    headers: headers(
                        "req-download-google-secret",
                        "Wed, 21 Oct 2015 07:28:10 GMT",
                    ),
                    body: Vec::new(),
                })],
            ),
            Phase::PollTransport => (
                vec![
                    reply(200, "req-submit-google-secret", json!({"name":OPERATION})),
                    Err(json_http::Error::transport("poll failed google-secret")),
                ],
                vec![],
            ),
            Phase::Deadline => (
                vec![reply(
                    200,
                    "req-submit-google-secret",
                    json!({"name":OPERATION}),
                )],
                vec![],
            ),
        };
        let (client, _) = scripted_client(json_responses, bytes);
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
        let generator = GoogleVideoGenerator::with_auth_and_runtime(
            client,
            "veo-current",
            Arc::new(StaticHeaderAuth::new(BTreeMap::from([(
                "x-goog-api-key".into(),
                "google-secret".into(),
            )]))),
            runtime,
        )
        .with_base_url("https://google.test/v1beta")
        .with_polling(Duration::from_secs(2), timeout)
        .with_clock(clock);
        let error = generator.generate(&request()).await.unwrap_err();
        assert_eq!(error.core().kind, kind, "{phase:?}");
        assert_eq!(error.core().provider_request_id.as_deref(), id, "{phase:?}");
        assert_eq!(
            error.core().retry_after,
            id.map(|_| Duration::from_secs(10)),
            "{phase:?}"
        );
        assert!(!error.to_string().contains("google-secret"), "{phase:?}");
        assert!(!format!("{error:?}").contains("google-secret"), "{phase:?}");
        let (actual_source, anchor) = match &error {
            VideoGenerationError::Internal { source, .. } => (
                if source.source_ref().is::<serde_json::Error>() {
                    "json"
                } else {
                    "http"
                },
                Some(source.defined_at().module_path()),
            ),
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
async fn rotating_video_auth_scrubs_each_phase_without_reapplying_the_hook() {
    let auth = Unimock::new(JsonHttpAuthMock::apply_headers.stub(|calls| {
        calls
            .call(matching!(_))
            .answers(&|_, headers: &mut BTreeMap<String, String>| {
                headers.insert("X-Custom-Key".into(), "Scheme submit-secret".into());
                Ok(())
            })
            .once()
            .then()
            .answers(&|_, headers: &mut BTreeMap<String, String>| {
                headers.insert("X-Custom-Key".into(), "Scheme poll-secret".into());
                Ok(())
            })
            .once();
    }));
    let (client, requests) = scripted_client(
        vec![
            Ok(JsonHttpResponse {
                status: 200,
                headers: Default::default(),
                body: json!({"name":OPERATION}),
            }),
            Ok(JsonHttpResponse {
                status: 200,
                headers: headers("req-poll-secret", "7"),
                body: json!({"name":OPERATION,"done":true,"error":{"code":7,"status":"PERMISSION_DENIED","message":"rejected submit-secret poll-secret"}}),
            }),
        ],
        vec![],
    );
    let generator = GoogleVideoGenerator::with_auth_and_runtime(
        client,
        "veo-current",
        Arc::new(auth),
        advancing_runtime().0,
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
        requests[0].headers.get("X-Custom-Key").map(String::as_str),
        Some("Scheme submit-secret")
    );
    assert_eq!(
        requests[1].headers.get("X-Custom-Key").map(String::as_str),
        Some("Scheme poll-secret")
    );
}
