//! Prefix-based nested histories retain equal but distinct failures.

use std::sync::Arc;

use ai_interface::{ErrorKind, Model, ModelAttempt, ModelError, ModelEvent};

use crate::{AnyError, FallbackModel};

use super::fixtures::{collect, model, request, stream, success};

fn repeated() -> ModelAttempt {
    ModelAttempt {
        provider: "same".to_owned(),
        model_id: "same".to_owned(),
        kind: ErrorKind::Transient,
        message: "same".to_owned(),
        emitted_events: false,
    }
}

fn repeated_error() -> ModelError {
    ModelError::transient("same", "same", "same")
}

#[tokio::test]
async fn nested_all_open_failures_keep_repeated_equal_records_on_success_and_failure() {
    for succeeds in [true, false] {
        let inner = FallbackModel::new(
            vec![
                model(vec![Err(repeated_error())]),
                model(vec![Err(repeated_error())]),
                model(vec![Err(repeated_error())]),
            ],
            Arc::new(AnyError),
        );
        let mut models = vec![model(vec![Err(repeated_error())]), Arc::new(inner)];
        if succeeds {
            models.push(model(vec![Ok(success("winner"))]));
        }
        let chain = FallbackModel::new(models, Arc::new(AnyError));
        let result = chain.complete(&request()).await;
        let actual = result
            .as_ref()
            .map(|r| r.attempts.clone())
            .unwrap_or_else(|e| e.attempts().to_vec());
        assert_eq!(actual, vec![repeated(); if succeeds { 4 } else { 3 }]);
        assert_eq!(
            result.as_ref().map(|r| r.text()).map_err(|e| e.core().kind),
            if succeeds {
                Ok("winner".to_owned())
            } else {
                Err(ErrorKind::Transient)
            }
        );
    }
}

#[tokio::test]
async fn nested_stream_history_skips_only_the_forwarded_prefix_and_keeps_equal_unseen_suffix() {
    let inner_error = repeated_error().with_attempts(vec![repeated(), repeated()]);
    let nested = model(vec![Ok(stream(vec![
        Ok(Some(ModelEvent::Fallback {
            error: repeated_error(),
            emitted_events: false,
        })),
        Err(inner_error),
    ]))]);
    let chain = FallbackModel::new(
        vec![nested, model(vec![Ok(success("winner"))])],
        Arc::new(AnyError),
    );
    let (result, events) = collect(&chain).await;
    let response = result.unwrap();
    assert_eq!(response.attempts, vec![repeated(); 3]);
    assert_eq!(events.len(), 6);
    assert_eq!(response.text(), "winner");
}

#[tokio::test]
async fn actual_nested_streams_forward_each_leaf_once_and_keep_complete_terminal_history() {
    for succeeds in [true, false] {
        let inner = FallbackModel::new(
            vec![
                model(vec![Err(repeated_error())]),
                model(vec![Err(repeated_error())]),
                model(vec![Ok(stream(vec![Err(repeated_error())]))]),
            ],
            Arc::new(AnyError),
        );
        let mut models: Vec<ai_interface::DynModel> = vec![Arc::new(inner)];
        if succeeds {
            models.push(model(vec![Ok(success("winner"))]));
        }
        let chain = FallbackModel::new(models, Arc::new(AnyError));
        let result = chain.complete(&request()).await;
        let history = result
            .as_ref()
            .map(|r| r.attempts.clone())
            .unwrap_or_else(|e| e.attempts().to_vec());
        assert_eq!(history, vec![repeated(); if succeeds { 3 } else { 2 }]);
    }
}

#[tokio::test]
async fn nested_success_forwards_original_fallback_events_without_duplicate_history() {
    let inner = FallbackModel::new(
        vec![
            model(vec![Err(repeated_error())]),
            model(vec![Ok(success("winner"))]),
        ],
        Arc::new(AnyError),
    );
    let outer = FallbackModel::new(
        vec![model(vec![Err(repeated_error())]), Arc::new(inner)],
        Arc::new(AnyError),
    );
    let mut stream = outer.stream(&request()).await.unwrap();
    let mut attempts = Vec::new();
    let mut flat_lengths = Vec::new();
    while let Some(event) = stream.next().await.unwrap() {
        if let ModelEvent::Fallback {
            error,
            emitted_events,
        } = event
        {
            flat_lengths.push(error.attempts().len());
            attempts.push(error.attempt(emitted_events));
        }
    }
    assert_eq!(attempts, vec![repeated(); 2]);
    assert_eq!(flat_lengths, vec![0, 0]);
    assert!(matches!(stream.next().await, Ok(None)));
}
