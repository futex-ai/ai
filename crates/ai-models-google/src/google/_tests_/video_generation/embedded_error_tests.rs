//! Embedded operation errors use canonical numeric meanings and actual HTTP headers.

use std::{collections::BTreeMap, sync::Arc, time::Duration};

use ai_interface::{ErrorKind, VideoGenerator};
use json_http::{JsonHttpResponse, StaticHeaderAuth};
use serde_json::json;

use crate::google::{
    error_context_support::{headers, unused_clock},
    video_generation::client::GoogleVideoGenerator,
};

use super::client_tests::{request, scripted_client, stationary_runtime};

const OPERATION: &str = "models/veo-current/operations/op_1";

#[tokio::test]
async fn embedded_operation_errors_keep_response_facts_and_precise_kinds() {
    for (code, status, kind) in [
        (16, "UNAUTHENTICATED", ErrorKind::RejectedKey),
        (7, "PERMISSION_DENIED", ErrorKind::RejectedKey),
        (5, "NOT_FOUND", ErrorKind::UnknownModel),
        (3, "INVALID_ARGUMENT", ErrorKind::InvalidRequest),
        (8, "RESOURCE_EXHAUSTED", ErrorKind::RateLimited),
        (14, "UNAVAILABLE", ErrorKind::Transient),
        (13, "INTERNAL", ErrorKind::Transient),
        (
            400,
            "context_length_exceeded",
            ErrorKind::ContextLimitExceeded,
        ),
    ] {
        let (client, _) = scripted_client(
            vec![Ok(JsonHttpResponse {
                status: 200,
                headers: headers("req-google-secret", "7"),
                body: json!({"name":OPERATION,"done":true,"error":{"code":code,"status":status,"message":"rejected google-secret"},"response":{"generateVideoResponse":{"raiMediaFilteredCount":1,"raiMediaFilteredReasons":["policy"]}}}),
            })],
            vec![],
        );
        let generator = GoogleVideoGenerator::with_auth_and_runtime(
            client,
            "veo-current",
            Arc::new(StaticHeaderAuth::new(BTreeMap::from([(
                "x-goog-api-key".into(),
                "google-secret".into(),
            )]))),
            stationary_runtime(),
        )
        .with_clock(unused_clock());
        let error = generator.generate(&request()).await.unwrap_err();
        assert_eq!(error.core().kind, kind, "{code}");
        assert_eq!(error.core().provider, "google");
        assert_eq!(error.core().model_id, "veo-current");
        assert_eq!(
            error.core().provider_request_id.as_deref(),
            Some("req-[redacted]"),
            "{code}"
        );
        assert_eq!(
            error.core().retry_after,
            Some(Duration::from_secs(7)),
            "{code}"
        );
        assert_eq!(error.core().message, "rejected [redacted]", "{code}");
        assert!(!format!("{error:?}").contains("google-secret"));
    }
}
