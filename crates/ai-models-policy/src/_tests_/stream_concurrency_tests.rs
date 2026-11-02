//! Deterministic lifetime checks for stream concurrency permits.

use std::{
    future::Future,
    pin::pin,
    sync::{
        Arc,
        atomic::{AtomicBool, AtomicUsize, Ordering},
    },
    task::{Context, Poll, Waker},
};

use ai_interface::{
    DynModelEventStream, ErrorKind, Model, ModelError, ModelEvent, ModelEventStream, ModelResult,
};
use async_trait::async_trait;

use crate::ConcurrencyLimitedModel;

use super::wrapper_fixtures::{request, scripted_model, stream, success};

#[derive(Clone, Copy, Debug)]
enum Release {
    Drop,
    DropAfterFinish,
    Eof,
    Error,
}

#[tokio::test]
async fn concurrency_holds_until_drop_eof_or_error_and_keeps_the_inner_stream_alive() {
    for release in [
        Release::Drop,
        Release::DropAfterFinish,
        Release::Eof,
        Release::Error,
    ] {
        let dropped = Arc::new(AtomicBool::new(false));
        let calls = Arc::new(AtomicUsize::new(0));
        let inner = match release {
            Release::Error => stream(vec![Err(ModelError::interrupted("p", "m", "closed"))]),
            _ => success("text"),
        };
        let model = ConcurrencyLimitedModel::new(
            scripted_model(
                vec![
                    Ok(Box::new(TrackedStream {
                        inner,
                        dropped: dropped.clone(),
                    })),
                    Ok(success("next")),
                ],
                calls.clone(),
            ),
            "mock",
            1,
        );
        let request = request();
        let mut first = model.stream(&request).await.unwrap();
        let mut second = pin!(model.stream(&request));
        let mut context = Context::from_waker(Waker::noop());

        assert!(
            matches!(second.as_mut().poll(&mut context), Poll::Pending),
            "release: {release:?}"
        );
        assert_eq!(calls.load(Ordering::SeqCst), 1);
        assert!(!dropped.load(Ordering::SeqCst));
        match release {
            Release::Drop => drop(first),
            Release::DropAfterFinish => {
                while !matches!(first.next().await.unwrap(), Some(ModelEvent::Finish { .. })) {}
                assert!(matches!(second.as_mut().poll(&mut context), Poll::Pending));
                assert!(!dropped.load(Ordering::SeqCst));
                drop(first);
            }
            Release::Eof => {
                while first.next().await.unwrap().is_some() {}
                assert!(matches!(first.next().await, Ok(None)));
            }
            Release::Error => {
                assert!(matches!(
                    first.next().await,
                    Err(ModelError::Failure { core, .. }) if core.kind == ErrorKind::Interrupted
                ));
                assert!(matches!(first.next().await, Ok(None)));
            }
        }
        assert!(dropped.load(Ordering::SeqCst));
        let _second = second.await.unwrap();
        assert_eq!(calls.load(Ordering::SeqCst), 2);
    }
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
