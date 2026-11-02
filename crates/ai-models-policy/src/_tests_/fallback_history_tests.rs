//! Complete earlier-attempt histories on terminal errors.

use std::{
    error::Error,
    sync::{Arc, Mutex},
};

use ai_interface::{ErrorCore, ErrorKind, Model, ModelAttempt, ModelError};
use unimock::{MockFn, Unimock, matching};

use crate::{AnyError, FallbackModel, FallbackPolicyMock};

use super::fixtures::{model, request, stream, text};

#[tokio::test]
async fn final_open_and_read_failures_keep_every_earlier_leaf_and_original_final_source() {
    for read_failure in [false, true] {
        let core = ErrorCore::new(ErrorKind::InvalidOutput, "last", "three", "last message");
        let final_error =
            ModelError::with_source(core.clone(), std::io::Error::other("final source"));
        let final_open = if read_failure {
            Ok(stream(vec![Err(final_error)]))
        } else {
            Err(final_error)
        };
        let chain = FallbackModel::new(
            vec![
                model(vec![Err(ModelError::invalid_request(
                    "first", "one", "rejected",
                ))]),
                model(vec![Ok(stream(vec![
                    Ok(Some(text("discard"))),
                    Err(ModelError::interrupted("second", "two", "closed")),
                ]))]),
                model(vec![final_open]),
            ],
            Arc::new(AnyError),
        );
        let error = chain.complete(&request()).await.unwrap_err();
        assert_eq!(
            error.attempts(),
            &[
                ModelAttempt {
                    provider: "first".to_owned(),
                    model_id: "one".to_owned(),
                    kind: ErrorKind::InvalidRequest,
                    message: "rejected".to_owned(),
                    emitted_events: false
                },
                ModelAttempt {
                    provider: "second".to_owned(),
                    model_id: "two".to_owned(),
                    kind: ErrorKind::Interrupted,
                    message: "closed".to_owned(),
                    emitted_events: true
                },
            ]
        );
        assert_eq!(error.core(), &core);
        assert_eq!(
            error
                .source()
                .unwrap()
                .downcast_ref::<std::io::Error>()
                .unwrap()
                .to_string(),
            "final source"
        );
    }
}

#[tokio::test]
async fn policy_stop_keeps_earlier_failures_for_both_open_and_read_paths() {
    for read_failure in [false, true] {
        let decisions = Arc::new(Mutex::new(Vec::new()));
        let captured = decisions.clone();
        let policy = Arc::new(Unimock::new(
            FallbackPolicyMock::should_fallback
                .some_call(matching!(_))
                .answers_arc(Arc::new(move |_, kind| {
                    captured.lock().unwrap().push(kind);
                    kind != ErrorKind::Interrupted
                })),
        ));
        let stop = ModelError::interrupted("stopped", "two", "stop");
        let stop = if read_failure {
            Ok(stream(vec![Err(stop)]))
        } else {
            Err(stop)
        };
        let chain = FallbackModel::new(
            vec![
                model(vec![Err(ModelError::transient("first", "one", "first"))]),
                model(vec![stop]),
                Arc::new(Unimock::new(())),
            ],
            policy,
        );
        let result = chain.complete(&request()).await;
        assert_eq!(
            result.as_ref().map(|r| r.text()).map_err(|e| e.core().kind),
            Err(ErrorKind::Interrupted)
        );
        let error = result.unwrap_err();
        assert_eq!(
            error.attempts(),
            &[ModelAttempt {
                provider: "first".to_owned(),
                model_id: "one".to_owned(),
                kind: ErrorKind::Transient,
                message: "first".to_owned(),
                emitted_events: false
            }]
        );
        assert_eq!(
            *decisions.lock().unwrap(),
            vec![ErrorKind::Transient, ErrorKind::Interrupted]
        );
        assert_eq!(
            (
                &error.core().provider,
                &error.core().model_id,
                &error.core().message
            ),
            (&"stopped".to_owned(), &"two".to_owned(), &"stop".to_owned())
        );
    }
}
