//! Response-scoped error facts and per-call authentication secrecy.

use std::{
    collections::BTreeMap,
    io,
    sync::{
        Arc,
        atomic::{AtomicU64, Ordering},
    },
    time::{Duration, SystemTime},
};

use ai_interface::{ErrorKind, JudgmentAnswerProblem, JudgmentError, ModelError};
use json_http::{JsonHttpAuthMock, JsonHttpHeaders};
use serde_json::json;
use unimock::{MockFn, Unimock, matching};

use crate::{ClockMock, ErrorContext, classify_json_http_error, classify_json_http_stream_error};

pub(super) fn context(provider: &str, model: &str) -> ErrorContext {
    ErrorContext::new(provider, model, &["request-id"], Arc::new(Unimock::new(())))
}

fn headers(id: &str, wait: &str) -> JsonHttpHeaders {
    let mut headers = JsonHttpHeaders::new();
    headers.insert("request-id", id.parse().unwrap());
    headers.insert("retry-after", wait.parse().unwrap());
    headers
}

#[test]
fn response_metadata_follows_the_failed_response_and_late_stream_errors() {
    let base = context("provider", "model");
    let stream_context = base.for_response(&headers("opened", "7"));
    let late =
        classify_json_http_stream_error(&stream_context, 1, json_http::Error::transport("closed"));
    assert_eq!(late.core().kind, ErrorKind::Interrupted);
    assert_eq!(late.core().retry_after, Some(Duration::from_secs(7)));
    assert_eq!(late.core().provider_request_id.as_deref(), Some("opened"));
    let failed = classify_json_http_stream_error(
        &stream_context,
        0,
        json_http::Error::HttpStatus {
            status: 429,
            headers: Box::new(headers("failed", "9")),
            body: json!({"error":{"message":"busy"}}),
        },
    );
    assert_eq!(failed.core().kind, ErrorKind::RateLimited);
    assert_eq!(failed.core().provider_request_id.as_deref(), Some("failed"));
    assert_eq!(failed.core().retry_after, Some(Duration::from_secs(9)));
    let wrapped = json_http::Error::InvalidSseContentType {
        content_type: Some("application/json".to_owned()),
    }
    .with_response_headers(headers("wrong-type", "3"));
    let failure = classify_json_http_stream_error(&base, 0, wrapped);
    assert_eq!(failure.core().kind, ErrorKind::Transient);
    assert_eq!(
        failure.core().provider_request_id.as_deref(),
        Some("wrong-type")
    );
    let ModelError::Internal { source, .. } = failure else {
        panic!("HTTP source lost");
    };
    assert!(matches!(
        source.downcast_ref::<json_http::Error>().unwrap().cause(),
        json_http::Error::InvalidSseContentType { .. }
    ));
    let buffered = stream_context.request_error(&json_http::Error::HttpStatus {
        status: 429,
        headers: Box::new(headers("buffered", "4")),
        body: json!({"error":{"message":"busy"}}),
    });
    assert_eq!(buffered.kind, ErrorKind::RateLimited);
    assert_eq!(buffered.provider_request_id.as_deref(), Some("buffered"));
    assert_eq!(buffered.retry_after, Some(Duration::from_secs(4)));
    let status = json_http::Error::HttpStatus {
        status: 429,
        headers: Box::new(headers("inner", "1")),
        body: json!({"message":"busy"}),
    }
    .with_response_headers(headers("outer", "2"));
    let outer = stream_context.request_error(&status);
    assert_eq!(outer.provider_request_id.as_deref(), Some("outer"));
    assert_eq!(outer.retry_after, Some(Duration::from_secs(2)));
    let unrelated = stream_context.request_error(&json_http::Error::transport("new phase failed"));
    assert_eq!(unrelated.provider_request_id, None);
    assert_eq!(unrelated.retry_after, None);
}

#[tokio::test]
async fn dynamic_auth_is_applied_once_without_wire_changes_and_secrets_stay_per_call() {
    let context = context("provider", "model");
    let auth = Arc::new(Unimock::new(
        JsonHttpAuthMock::apply_headers
            .next_call(matching!(_))
            .answers_arc(Arc::new(|_, headers: &mut BTreeMap<String, String>| {
                headers.insert(
                    "Authorization".to_owned(),
                    "Bearer rotating-secret".to_owned(),
                );
                headers.insert("custom-key".to_owned(), "custom-secret".to_owned());
                headers.insert(
                    "compound-key".to_owned(),
                    "Scheme prefix secret-token".to_owned(),
                );
                Ok(())
            })),
    ));
    let mut transmitted = BTreeMap::new();
    context
        .capture_auth(auth)
        .apply_headers(&mut transmitted)
        .await
        .unwrap();
    assert_eq!(
        transmitted.get("Authorization").map(String::as_str),
        Some("Bearer rotating-secret")
    );
    assert_eq!(
        transmitted.get("custom-key").map(String::as_str),
        Some("custom-secret")
    );
    assert_eq!(
        context.redact("Bearer rotating-secret custom-secret"),
        "[redacted] [redacted]"
    );
    assert_eq!(
        transmitted.get("compound-key").map(String::as_str),
        Some("Scheme prefix secret-token")
    );
    assert_eq!(context.redact("secret-token"), "[redacted]");
    let response = context.for_response(&headers("rotating-secret", "7"));
    let error = classify_json_http_error(
        &response,
        401,
        &headers("rotating-secret", "7"),
        &json!({"error":{"message":"Bearer rotating-secret custom-secret"}}),
    );
    assert_eq!(error.core().kind, ErrorKind::RejectedKey);
    assert_eq!(
        error.core().provider_request_id.as_deref(),
        Some("[redacted]")
    );
    for text in [error.to_string(), format!("{error:?}")] {
        assert!(!text.contains("rotating-secret"), "{text}");
        assert!(!text.contains("custom-secret"), "{text}");
    }
    let source = io::Error::other("rotating-secret");
    let error = response.apply_model_error(ModelError::with_source(error.core().clone(), source));
    assert!(!format!("{error:?}").contains("rotating-secret"));
    let ModelError::Internal { source, .. } = error else {
        panic!("source lost");
    };
    assert_eq!(
        source.downcast_ref::<io::Error>().unwrap().to_string(),
        "rotating-secret"
    );
    let failing = Arc::new(Unimock::new(
        JsonHttpAuthMock::apply_headers
            .next_call(matching!(_))
            .returns(Err(json_http::Error::transport("unknown-secret"))),
    ));
    let failure = context
        .capture_auth(failing)
        .apply_headers(&mut BTreeMap::new())
        .await
        .unwrap_err();
    assert_eq!(
        context.request_error(&failure).message,
        "authentication hook failed"
    );
    let other = ErrorContext::new(
        "provider",
        "model",
        &["request-id"],
        Arc::new(Unimock::new(())),
    );
    assert_eq!(other.redact("rotating-secret"), "rotating-secret");
}

#[test]
fn auth_diagnostics_and_provider_controlled_judgment_fields_are_safe() {
    let context = context("provider", "model");
    context.add_secret("token");
    context.add_secret("token-longer");
    let auth = context.request_error(&json_http::Error::auth(
        "Authorization: Bearer unknown-secret",
    ));
    assert_eq!(auth.message, "authentication hook failed");
    assert_eq!(
        context.redact("token-longer token"),
        "[redacted] [redacted]"
    );
    for problem in [
        JudgmentAnswerProblem::UnknownOption {
            label: "token".to_owned(),
        },
        JudgmentAnswerProblem::MissingOption {
            label: "token-longer".to_owned(),
        },
    ] {
        let error = context.apply_judgment_error(JudgmentError::invalid_answer(
            "local",
            "validation",
            "token",
            problem,
        ));
        assert_eq!(error.core().provider, "provider");
        assert_eq!(error.core().model_id, "model");
        assert!(!format!("{error:?}").contains("token"));
        assert!(!error.to_string().contains("token"));
        let JudgmentError::InvalidAnswer { id, problem, .. } = error else {
            panic!("typed problem lost");
        };
        assert_eq!(id, "[redacted]");
        let label = match problem {
            JudgmentAnswerProblem::UnknownOption { label }
            | JudgmentAnswerProblem::MissingOption { label } => label,
            _ => panic!("problem kind changed"),
        };
        assert_eq!(label, "[redacted]");
    }
}

#[test]
fn response_retry_dates_are_evaluated_when_the_error_occurs() {
    let now = Arc::new(AtomicU64::new(5));
    let clock = Arc::new(Unimock::new(
        ClockMock::now.some_call(matching!()).answers_arc({
            let now = now.clone();
            Arc::new(move |_| {
                SystemTime::UNIX_EPOCH + Duration::from_secs(now.load(Ordering::SeqCst))
            })
        }),
    ));
    let context = ErrorContext::new("provider", "model", &["request-id"], clock);
    let response = context.for_response(&headers("late", "Thu, 01 Jan 1970 00:00:10 GMT"));
    now.store(20, Ordering::SeqCst);
    let error = response.model_error(ErrorKind::RateLimited, "busy");
    assert_eq!(error.core().retry_after, Some(Duration::ZERO));
}
