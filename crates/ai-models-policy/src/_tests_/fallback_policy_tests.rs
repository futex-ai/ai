//! Required fallback presets and caller-selected stop rules.

use std::{
    error::Error,
    sync::{
        Arc, Mutex,
        atomic::{AtomicUsize, Ordering},
    },
    time::Duration,
};

use ai_interface::{ErrorCore, ErrorKind, Model, ModelError};
use unimock::{MockFn, Unimock, matching};

use crate::{AnotherModelCanSucceed, AnyError, FallbackModel, FallbackPolicy, FallbackPolicyMock};

use super::{
    fixtures::{request, stream, success},
    request_fixtures::{capturing_model, controlled_request},
    retry_fixtures::KINDS,
};

#[test]
fn explicit_presets_follow_every_error_kind_and_both_allow_interrupted() {
    for (kind, another, _) in KINDS {
        assert!(AnyError.should_fallback(kind), "{kind:?}");
        assert_eq!(
            AnotherModelCanSucceed.should_fallback(kind),
            another,
            "{kind:?}"
        );
    }
}

#[tokio::test]
async fn presets_apply_the_kind_table_to_open_and_read_failures() {
    for (kind, another, _) in KINDS {
        for (policy, allowed) in [
            (Arc::new(AnyError) as Arc<dyn FallbackPolicy>, true),
            (Arc::new(AnotherModelCanSucceed), another),
        ] {
            for read_failure in [false, true] {
                let error = ModelError::failure(kind, "first", "model", "safe original");
                let first = if read_failure {
                    Ok(stream(vec![Err(error)]))
                } else {
                    Err(error)
                };
                let calls = Arc::new(AtomicUsize::new(0));
                let original = controlled_request();
                let received = Arc::new(Mutex::new(Vec::new()));
                let chain = FallbackModel::new(
                    vec![
                        capturing_model(
                            vec![first],
                            received.clone(),
                            Arc::new(AtomicUsize::new(0)),
                        ),
                        if allowed {
                            capturing_model(
                                vec![Ok(success("winner"))],
                                received.clone(),
                                calls.clone(),
                            )
                        } else {
                            Arc::new(Unimock::new(()))
                        },
                    ],
                    policy.clone(),
                );
                let result = chain.complete(&original).await;
                assert_eq!(
                    result.as_ref().map(|r| r.text()).map_err(|e| e.core().kind),
                    if allowed {
                        Ok("winner".to_owned())
                    } else {
                        Err(kind)
                    },
                    "{kind:?}, read={read_failure}"
                );
                assert_eq!(calls.load(Ordering::SeqCst), usize::from(allowed));
                assert_eq!(
                    *received.lock().unwrap(),
                    vec![original; if allowed { 2 } else { 1 }]
                );
            }
        }
    }
}

#[tokio::test]
async fn custom_policy_stops_on_interrupted_and_preserves_core_source_and_full_request() {
    let mut original = request();
    original.controls.execution.total_timeout = Some(Duration::from_secs(17));
    original.controls.generation.max_output_tokens = Some(91);
    let mut core = ErrorCore::new(ErrorKind::Interrupted, "provider", "model", "original");
    core.retry_after = Some(Duration::from_secs(7));
    core.provider_request_id = Some("req-123".to_owned());
    let expected_core = core.clone();
    let received = Arc::new(Mutex::new(Vec::new()));
    let captured = received.clone();
    let first = Arc::new(Unimock::new(
        ai_interface::ModelMock::stream
            .some_call(matching!(_))
            .answers_arc(Arc::new(move |_, request: &ai_interface::ModelRequest| {
                captured.lock().unwrap().push(request.clone());
                Err(ModelError::with_source(
                    core.clone(),
                    std::io::Error::other("original source"),
                ))
            })),
    ));
    let policy = Arc::new(Unimock::new(
        FallbackPolicyMock::should_fallback
            .some_call(matching!(ErrorKind::Interrupted))
            .returns(false),
    ));
    let chain = FallbackModel::new(vec![first, Arc::new(Unimock::new(()))], policy);
    let result = chain.complete(&original).await;
    assert_eq!(
        result
            .as_ref()
            .map(|r| r.text())
            .map_err(|e| e.core().clone()),
        Err(expected_core)
    );
    let error = result.unwrap_err();
    assert_eq!(
        error
            .source()
            .unwrap()
            .downcast_ref::<std::io::Error>()
            .unwrap()
            .to_string(),
        "original source"
    );
    assert!(error.attempts().is_empty());
    assert_eq!(*received.lock().unwrap(), vec![original]);
}

#[tokio::test]
async fn empty_explicit_policy_fallback_fails_every_call_with_local_internal_kind() {
    let chain = FallbackModel::new(Vec::new(), Arc::new(AnyError));
    for _ in 0..2 {
        let result = chain.complete(&request()).await;
        let error = result.unwrap_err();
        assert_eq!(error.core().kind, ErrorKind::Internal);
        assert_eq!(error.core().provider, "local");
        assert_eq!(error.core().model_id, "fallback");
        assert!(error.attempts().is_empty());
    }
}
