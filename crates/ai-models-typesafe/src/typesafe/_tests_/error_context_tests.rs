//! Buffered status, output, and read failures retain their actual response facts and sources.

use std::{
    sync::Arc,
    time::{Duration, SystemTime},
};

use ai_interface::{ErrorKind, JudgmentError, JudgmentModel, SafeErrorSource};
use ai_models_core::{Clock, ClockMock};
use json_http::JsonHttpHeaders;
use serde_json::{Value, json};
use unimock::{MockFn, Unimock, matching};

use super::{
    TypeSafeJudgmentModel,
    test_support::{json_response, recording_http_client, scripted_http_client, simple_request},
};

const KEY: &str = "typesafe-response-secret";

#[tokio::test]
async fn status_and_output_errors_keep_metadata_safe_diagnostics_and_original_sources() {
    for (status, body, retry, kind, serde_source, now) in [
        (
            429,
            json!({"detail":{"message":"rejected typesafe-response-secret"}}),
            "Wed, 21 Oct 2015 07:28:10 GMT",
            ErrorKind::RateLimited,
            false,
            Some(SystemTime::UNIX_EPOCH + Duration::from_secs(1_445_412_480)),
        ),
        (
            400,
            json!({"error":{"code":"context_length_exceeded"},"detail":"too much context"}),
            "5",
            ErrorKind::ContextLimitExceeded,
            false,
            None,
        ),
        (
            400,
            json!({"detail":{"error_type":"context_length_exceeded","message":"too much context"}}),
            "5",
            ErrorKind::ContextLimitExceeded,
            false,
            None,
        ),
        (
            400,
            json!({"detail":{"error_type":"model_context_window_exceeded","message":"too much context"}}),
            "5",
            ErrorKind::ContextLimitExceeded,
            false,
            None,
        ),
        (
            400,
            json!({"detail":{"error_type":"input_too_long","message":"too much context"}}),
            "5",
            ErrorKind::ContextLimitExceeded,
            false,
            None,
        ),
        (
            400,
            json!({"detail":{"error_type":"too_many_tokens","message":"too much context"}}),
            "5",
            ErrorKind::ContextLimitExceeded,
            false,
            None,
        ),
        (
            422,
            json!({"detail":{"error_type":"context_length_exceeded","message":"schema rejected"}}),
            "7",
            ErrorKind::InvalidRequest,
            false,
            None,
        ),
        (
            200,
            json!({"model":"jev-1.13.0","answers":{"is_urgent":{"type":"typesafe-response-secret"}}}),
            "7",
            ErrorKind::InvalidOutput,
            true,
            None,
        ),
    ] {
        let clock: Arc<dyn Clock> = match now {
            Some(now) => Arc::new(Unimock::new(
                ClockMock::now.some_call(matching!()).returns(now),
            )),
            None => Arc::new(Unimock::new(())),
        };
        let mut response = json_response(status, body);
        response.headers = headers("req-typesafe-response-secret", retry);
        let (http, requests) = recording_http_client(response);
        let error = TypeSafeJudgmentModel::new(http, "jev-unlisted", KEY)
            .with_clock(clock)
            .judge(&simple_request())
            .await
            .unwrap_err();
        assert_eq!(error.core().kind, kind);
        assert_eq!(error.core().provider, "typesafe");
        assert_eq!(error.core().model_id, "jev-unlisted");
        assert_eq!(
            error.core().provider_request_id.as_deref(),
            Some("req-[redacted]")
        );
        assert_eq!(
            error.core().retry_after,
            Some(Duration::from_secs(match status {
                429 => 10,
                400 => 5,
                _ => 7,
            }))
        );
        assert!(!error.to_string().contains(KEY));
        assert!(!format!("{error:?}").contains(KEY));
        let source_matches = match &error {
            JudgmentError::Internal { source, .. } => {
                source
                    .source_ref()
                    .downcast_ref::<SafeErrorSource>()
                    .unwrap()
                    .raw_cause()
                    .is::<serde_json::Error>()
                    && source.defined_at().module_path() == "ai_interface::judgment::error"
                    && source.caller_at().file().ends_with("typesafe/response.rs")
                    && source.caller_at().line() > 0
            }
            _ => false,
        };
        assert_eq!(source_matches, serde_source);
        let requests = requests.lock().unwrap();
        assert_eq!(requests.len(), 1);
        assert_eq!(
            requests[0].headers.get("Authorization").map(String::as_str),
            Some("Bearer typesafe-response-secret")
        );
    }
}

#[tokio::test]
async fn request_failures_keep_their_headers_kinds_and_tracked_original_sources() {
    let serialized = serde_json::from_str::<Value>("{").unwrap_err();
    let decoded = serde_json::from_str::<Value>("{").unwrap_err();
    for (failure, kind, id, wait) in [
        (
            json_http::Error::SerializeRequest { source: serialized },
            ErrorKind::Internal,
            None,
            None,
        ),
        (
            json_http::Error::DeserializeResponse {
                body: Value::Null,
                source: decoded,
            }
            .with_response_headers(headers("req-decode", "4")),
            ErrorKind::InvalidOutput,
            Some("req-decode"),
            Some(4),
        ),
        (
            json_http::Error::HttpStatus {
                status: 403,
                headers: Box::new(headers("req-inner", "1")),
                body: json!({"detail":"denied typesafe-response-secret"}),
            }
            .with_response_headers(headers("req-outer", "9")),
            ErrorKind::RejectedKey,
            Some("req-outer"),
            Some(9),
        ),
        (
            json_http::Error::transport("network failed typesafe-response-secret"),
            ErrorKind::Transient,
            None,
            None,
        ),
    ] {
        let (http, _) = scripted_http_client(vec![Err(failure)]);
        let error = TypeSafeJudgmentModel::new(http, "jev-latest", KEY)
            .with_clock(Arc::new(Unimock::new(())))
            .judge(&simple_request())
            .await
            .unwrap_err();
        assert_eq!(error.core().kind, kind);
        assert_eq!(error.core().provider, "typesafe");
        assert_eq!(error.core().model_id, "jev-latest");
        assert_eq!(error.core().provider_request_id.as_deref(), id);
        assert_eq!(error.core().retry_after, wait.map(Duration::from_secs));
        assert!(!error.to_string().contains(KEY));
        assert!(!format!("{error:?}").contains(KEY));
        let JudgmentError::Internal { source, .. } = &error else {
            panic!("original source lost")
        };
        assert!(
            source
                .source_ref()
                .downcast_ref::<SafeErrorSource>()
                .unwrap()
                .raw_cause()
                .is::<json_http::Error>()
        );
        assert_eq!(
            source.defined_at().module_path(),
            "ai_interface::judgment::error"
        );
        assert!(source.caller_at().file().ends_with("typesafe/error.rs"));
        assert!(source.caller_at().line() > 0);
    }
}

pub(super) fn headers(id: &str, retry: &str) -> JsonHttpHeaders {
    let mut headers = JsonHttpHeaders::new();
    headers.insert("x-request-id", id.parse().unwrap());
    headers.insert("retry-after", retry.parse().unwrap());
    headers
}
