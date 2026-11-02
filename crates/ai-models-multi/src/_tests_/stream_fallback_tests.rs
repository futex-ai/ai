//! Failed attempt reporting and terminal behavior of fallback streams.

use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};

use ai_interface::{
    DynModelEventStream, FinishReason, Model, ModelAttempt, ModelAttemptError, ModelError,
    ModelEvent, ModelEventStream, ModelResult, ModelUsage,
};
use async_trait::async_trait;
use unimock::Unimock;

use crate::MultiModel;

use super::fixtures::{Observed, collect, model, request, stream, success, text};

#[tokio::test]
async fn fallback_records_every_failed_open_before_the_first_event() {
    let model = MultiModel::new(vec![
        model(vec![Err(ModelError::provider("first", "one", "rejected"))]),
        model(vec![Err(ModelError::rate_limited("second", "two", "busy"))]),
        model(vec![Ok(success("done"))]),
    ]);
    let (response, events) = collect(&model).await;

    assert!(matches!(&response, Ok(response) if response.text() == "done"));
    assert_eq!(
        events,
        vec![
            Observed::Fallback(ModelAttempt {
                error: ModelAttemptError::Provider {
                    provider: "first".to_owned(),
                    model_id: "one".to_owned(),
                    message: "rejected".to_owned()
                }
            }),
            Observed::Fallback(ModelAttempt {
                error: ModelAttemptError::RateLimited {
                    provider: "second".to_owned(),
                    model_id: "two".to_owned(),
                    message: "busy".to_owned()
                }
            }),
            Observed::Text(0, "done".to_owned()),
            Observed::Usage(ModelUsage::default()),
            Observed::Finish(FinishReason::Stop, "mock".to_owned(), "mock".to_owned()),
        ]
    );
    assert_eq!(response.unwrap().attempts.len(), 2);
}

#[tokio::test]
async fn fallback_moves_after_a_read_failure_and_failed_later_opens() {
    let model = MultiModel::new(vec![
        model(vec![Ok(stream(vec![
            Ok(Some(text("discarded"))),
            Err(ModelError::interrupted("first", "one", "closed")),
        ]))]),
        model(vec![Err(ModelError::provider("second", "two", "rejected"))]),
        model(vec![Ok(success("done"))]),
    ]);
    let (response, events) = collect(&model).await;

    assert!(matches!(&response, Ok(response) if response.text() == "done"));
    assert_eq!(
        events,
        vec![
            Observed::Text(0, "discarded".to_owned()),
            Observed::Fallback(ModelAttempt {
                error: ModelAttemptError::Interrupted {
                    provider: "first".to_owned(),
                    model_id: "one".to_owned(),
                    message: "closed".to_owned()
                }
            }),
            Observed::Fallback(ModelAttempt {
                error: ModelAttemptError::Provider {
                    provider: "second".to_owned(),
                    model_id: "two".to_owned(),
                    message: "rejected".to_owned()
                }
            }),
            Observed::Text(0, "done".to_owned()),
            Observed::Usage(ModelUsage::default()),
            Observed::Finish(FinishReason::Stop, "mock".to_owned(), "mock".to_owned()),
        ]
    );
    assert_eq!(response.unwrap().attempts.len(), 2);
}

#[tokio::test]
async fn fallback_returns_the_last_error_and_stays_terminal() {
    let empty = MultiModel::default().stream(&request()).await;
    assert!(matches!(empty, Err(ModelError::Internal { .. })));

    let all_failed = MultiModel::new(vec![
        model(vec![Err(ModelError::provider("first", "one", "first"))]),
        model(vec![Err(ModelError::provider("last", "two", "last"))]),
    ])
    .stream(&request())
    .await;
    assert!(
        matches!(all_failed, Err(ModelError::Provider { provider, message, .. }) if provider == "last" && message == "last")
    );

    let model = MultiModel::new(vec![model(vec![Ok(stream(vec![Err(
        ModelError::interrupted("last", "one", "closed"),
    )]))])]);
    let mut stream = model.stream(&request()).await.unwrap();
    assert!(
        matches!(stream.next().await, Err(ModelError::Interrupted { provider, message, .. }) if provider == "last" && message == "closed")
    );
    assert!(matches!(stream.next().await, Ok(None)));
    assert!(matches!(stream.next().await, Ok(None)));
}

#[tokio::test]
async fn dropping_fallback_closes_the_inner_stream_without_opening_another_lane() {
    let dropped = Arc::new(AtomicBool::new(false));
    let model = MultiModel::new(vec![
        model(vec![Ok(Box::new(TrackedStream {
            inner: success("partial"),
            dropped: dropped.clone(),
        }))]),
        Arc::new(Unimock::new(())),
    ]);
    let mut stream = model.stream(&request()).await.unwrap();
    assert!(
        matches!(stream.next().await, Ok(Some(ModelEvent::TextDelta { delta, .. })) if delta == "partial")
    );
    drop(stream);
    assert!(dropped.load(Ordering::SeqCst));
}

struct TrackedStream {
    inner: DynModelEventStream,
    dropped: Arc<AtomicBool>,
}

#[async_trait]
impl ModelEventStream for TrackedStream {
    async fn next(&mut self) -> ModelResult<Option<ModelEvent>> {
        self.inner.next().await
    }
}

impl Drop for TrackedStream {
    fn drop(&mut self) {
        self.dropped.store(true, Ordering::SeqCst);
    }
}
