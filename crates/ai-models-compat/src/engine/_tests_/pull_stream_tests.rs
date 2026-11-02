//! Pull stream ordering, terminal validation, and closed-stream regressions.

use ai_interface::{ConversationMessage, Model, ModelError, ModelRequest, StructuredOutputSchema};
use ai_models_core::test_support::{
    SseFixture, StreamItem, done_event, event, recording_streaming_client,
};
use serde_json::json;

use crate::{deepseek::DeepSeekModel, stream_test_support::portable_events};

#[tokio::test]
async fn streams_tool_events_before_usage_and_finish() {
    let model = model(vec![
        event(json!({"choices": [{"index": 0, "delta": {
            "reasoning_content": "think", "content": "checking",
            "tool_calls": [{"index": 0, "id": "call-1", "function": {
                "name": "lookup", "arguments": "{\"city\":"
            }}]
        }}]})),
        event(json!({"choices": [{"index": 0, "delta": {
            "tool_calls": [{"index": 0, "function": {"arguments": "\"Paris\"}"}}]
        }, "finish_reason": "tool_calls"}]})),
        usage(),
        done_event(),
    ]);
    let mut stream = model.stream(&simple_request()).await.expect("stream opens");
    let mut events = Vec::new();
    while let Some(event) = stream.next().await.expect("valid stream") {
        events.push(serde_json::to_value(event).expect("event serializes"));
    }

    let events = portable_events(&events);
    assert_eq!(events.len(), 8);
    assert_eq!(
        events[..6],
        [
            json!({"ReasoningDelta": {"part_index": 0, "delta": "think"}}),
            json!({"TextDelta": {"part_index": 1, "delta": "checking"}}),
            json!({"ToolCallStart": {"part_index": 2, "id": "call-1", "name": "lookup"}}),
            json!({"ToolCallArgumentsDelta": {"part_index": 2, "delta": "{\"city\":"}}),
            json!({"ToolCallArgumentsDelta": {"part_index": 2, "delta": "\"Paris\"}"}}),
            json!({"ToolCallEnd": {"part_index": 2}}),
        ]
    );
    assert_eq!(events[6]["Usage"]["usage"]["total_tokens"], 5);
    assert_eq!(events[7]["Finish"]["finish_reason"], "ToolCalls");
    assert!(
        stream
            .next()
            .await
            .expect("stream remains closed")
            .is_none()
    );
}

#[tokio::test]
async fn invalid_tool_arguments_fail_before_usage_and_close_the_stream() {
    let model = model(vec![
        event(json!({"choices": [{"index": 0, "delta": {
            "reasoning_content": "think",
            "tool_calls": [{"index": 0, "id": "call-1", "function": {
                "name": "lookup", "arguments": "{bad"
            }}]
        }, "finish_reason": "tool_calls"}]})),
        usage(),
        done_event(),
    ]);
    let mut stream = model.stream(&simple_request()).await.expect("stream opens");
    let mut content = Vec::new();
    let result = loop {
        match stream.next().await {
            Ok(Some(event)) => content.push(serde_json::to_value(event).expect("event serializes")),
            Ok(None) => break Ok(()),
            Err(error) => break Err(error),
        }
    };

    assert!(matches!(result, Err(ModelError::Provider { .. })));
    assert_eq!(content.len(), 3);
    assert!(content.iter().all(|event| event.get("Usage").is_none()));
    assert!(content.iter().all(|event| event.get("Finish").is_none()));
    assert!(
        stream
            .next()
            .await
            .expect("failed stream stays closed")
            .is_none()
    );
}

#[tokio::test]
async fn uses_the_last_finish_reason_and_ignores_other_choices() {
    let model = model(vec![
        event(json!({"choices": [
            {"index": 0, "delta": {"content": "first"}, "finish_reason": "length"},
            {"index": 1, "delta": {"content": "ignored"}}
        ]})),
        event(json!({"choices": [{"index": 0, "delta": {}, "finish_reason": "stop"}]})),
        usage(),
        done_event(),
    ]);
    let mut stream = model.stream(&simple_request()).await.expect("stream opens");
    let mut events = Vec::new();
    while let Some(event) = stream.next().await.expect("last finish wins") {
        events.push(serde_json::to_value(event).expect("event serializes"));
    }

    assert_eq!(
        events[0],
        json!({"TextDelta": {"part_index": 0, "delta": "first"}})
    );
    assert_eq!(events[2]["Finish"]["finish_reason"], "Stop");
    assert_eq!(events.len(), 3);
}

#[tokio::test]
async fn schema_failure_keeps_text_and_emits_no_terminal_events() {
    let model = model(vec![
        event(
            json!({"choices": [{"index": 0, "delta": {"content": "{\"count\":\"bad\"}"}, "finish_reason": "stop"}]}),
        ),
        usage(),
        done_event(),
    ]);
    let mut request = simple_request();
    request.response_schema = Some(StructuredOutputSchema {
        name: "count".to_owned(),
        schema: json!({"type": "object", "properties": {"count": {"type": "integer"}}, "required": ["count"]}),
    });
    let mut stream = model.stream(&request).await.expect("stream opens");
    let first = stream
        .next()
        .await
        .expect("schema text is visible")
        .expect("text event");

    assert_eq!(
        serde_json::to_value(first).expect("event serializes"),
        json!({"TextDelta": {"part_index": 0, "delta": "{\"count\":\"bad\"}"}})
    );
    assert!(matches!(
        stream.next().await,
        Err(ModelError::Provider { .. })
    ));
    assert!(
        stream
            .next()
            .await
            .expect("failed stream stays closed")
            .is_none()
    );
}

fn model(events: Vec<StreamItem>) -> DeepSeekModel {
    let (client, _) = recording_streaming_client(vec![SseFixture::Stream(events)]);
    DeepSeekModel::new(client, "key")
}

fn usage() -> StreamItem {
    event(
        json!({"choices": [], "usage": {"prompt_tokens": 2, "completion_tokens": 3, "total_tokens": 5}}),
    )
}

fn simple_request() -> ModelRequest {
    ModelRequest {
        system_prompt: String::new(),
        messages: vec![ConversationMessage::user("hello")],
        tools: Vec::new(),
        response_schema: None,
        controls: Default::default(),
    }
}
