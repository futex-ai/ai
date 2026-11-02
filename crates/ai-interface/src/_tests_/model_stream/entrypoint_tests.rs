//! Provided completion fold, model mocks, and built-in mock stream behavior.

use std::io::{Error, ErrorKind};

use serde_json::json;
use unimock::{MockFn, Unimock, matching};

use crate::{FinishReason, MockModel, Model, ModelError, ModelMock, StructuredOutputSchema};

use super::fixtures::{finish, model_for, request, text, usage};

#[tokio::test]
async fn complete_folds_the_mocked_event_stream_through_the_public_traits() {
    let model = model_for(vec![
        Ok(Some(text(0, "hello"))),
        Ok(Some(text(0, " world"))),
        Ok(Some(usage())),
        Ok(Some(finish(FinishReason::Stop))),
        Ok(None),
    ]);

    let response = model.complete(&request()).await.unwrap();

    assert_eq!(response.assistant_message, "hello world");
    assert_eq!(response.usage.total_tokens, 10);
}

#[tokio::test]
async fn complete_preserves_the_original_open_and_read_errors() {
    let model = Unimock::new(
        ModelMock::stream
            .some_call(matching!(_))
            .answers(&|_, _| Err(ModelError::rate_limited("open", "model", "busy"))),
    );
    let open = model.complete(&request()).await;
    assert!(matches!(
        open,
        Err(ModelError::RateLimited { provider, model_id, message })
        if provider == "open" && model_id == "model" && message == "busy"
    ));

    let model = model_for(vec![
        Ok(Some(text(0, "partial"))),
        Err(ModelError::interrupted("read", "model", "closed")),
    ]);
    let read = model.complete(&request()).await;
    assert!(matches!(
        read,
        Err(ModelError::Interrupted { provider, model_id, message })
        if provider == "read" && model_id == "model" && message == "closed"
    ));

    let model = model_for(vec![Err(ModelError::internal(Error::new(
        ErrorKind::ConnectionReset,
        "original source",
    )))]);
    let internal = model.complete(&request()).await;
    let Err(ModelError::Internal { source }) = internal else {
        panic!("the original internal error must reach the caller");
    };
    assert_eq!(
        source.downcast_ref::<Error>().map(Error::kind),
        Some(ErrorKind::ConnectionReset)
    );
}

#[tokio::test]
async fn mock_stream_emits_schema_text_usage_finish_and_stays_exhausted() {
    let mut request = request();
    request.response_schema = Some(StructuredOutputSchema {
        name: "answer".to_owned(),
        schema: json!({"const": {"accepted": true}}),
    });
    let mut stream = MockModel::new("mock-dev").stream(&request).await.unwrap();
    let mut events = Vec::new();
    while let Some(event) = stream.next().await.unwrap() {
        events.push(serde_json::to_value(event).unwrap());
    }

    assert_eq!(
        events,
        vec![
            json!({"TextDelta": {"part_index": 0, "delta": "{\"accepted\":true}"}}),
            json!({"Usage": {"usage": {
                "input_tokens": 7, "output_tokens": 1, "cached_input_tokens": 0,
                "reasoning_tokens": 0, "total_tokens": 8, "estimated_cost_microusd": 0
            }}}),
            json!({"Finish": {
                "finish_reason": "Stop", "provider": "mock", "model_id": "mock-dev",
                "structured_output": {"accepted": true}
            }}),
        ]
    );
    assert!(matches!(stream.next().await, Ok(None)));
    assert!(matches!(stream.next().await, Ok(None)));
}
