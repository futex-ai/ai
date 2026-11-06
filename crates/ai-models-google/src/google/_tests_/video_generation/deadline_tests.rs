//! Every held video phase reports expiry through the non-retryable kind.

use std::{error::Error, time::Duration};

use ai_interface::{ErrorKind, SafeErrorSource, VideoGenerationError, VideoGenerator};
use json_http::JsonHttpMethod;

use super::{
    client_tests::request,
    deadline_support::{Failure, Phase, fixture},
};

#[tokio::test]
async fn held_submission_past_total_deadline_is_timed_out() {
    assert_timed_out(Phase::Submission, &[10]).await;
}

#[tokio::test]
async fn held_later_poll_past_total_deadline_is_timed_out() {
    assert_timed_out(Phase::Poll, &[10, 8, 6]).await;
}

#[tokio::test]
async fn held_download_past_total_deadline_is_timed_out() {
    assert_timed_out(Phase::Download, &[10, 8, 7]).await;
}

#[tokio::test]
async fn http_500_before_total_deadline_keeps_transient_kind() {
    for failure in [Failure::BufferedStatus, Failure::StatusError] {
        let (generator, requests) = fixture(Phase::Submission, failure);
        let error = generator.generate(&request()).await.unwrap_err();
        assert_eq!(error.core().kind, ErrorKind::Transient);
        assert_eq!(requests.lock().unwrap().len(), 1);
    }
}

async fn assert_timed_out(phase: Phase, timeout_seconds: &[u64]) {
    let (generator, requests) = fixture(phase, Failure::Transport);
    let error = generator.generate(&request()).await.unwrap_err();
    assert_eq!(error.core().kind, ErrorKind::TimedOut);
    assert_eq!(error.core().provider.as_str(), "google");
    assert_eq!(error.core().model_id, "veo-current");
    let requests = requests.lock().unwrap();
    assert_eq!(
        requests
            .iter()
            .map(|request| request.timeout)
            .collect::<Vec<_>>(),
        timeout_seconds
            .iter()
            .copied()
            .map(Duration::from_secs)
            .collect::<Vec<_>>()
    );
    assert_eq!(
        requests
            .iter()
            .filter(|request| request.method == JsonHttpMethod::Post)
            .count(),
        1
    );
    let VideoGenerationError::Internal { source, .. } = &error else {
        panic!("transport cause missing")
    };
    let source = source
        .source_ref()
        .downcast_ref::<SafeErrorSource>()
        .unwrap();
    assert!(
        source
            .raw_cause()
            .downcast_ref::<json_http::Error>()
            .unwrap()
            .is_transport_failure()
    );
    let mut source = Some(&error as &dyn Error);
    while let Some(link) = source {
        assert!(!link.to_string().contains("placeholder-video-key"));
        assert!(!format!("{link:?}").contains("placeholder-video-key"));
        source = link.source();
    }
}
