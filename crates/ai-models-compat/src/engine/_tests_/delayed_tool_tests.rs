//! Delayed tool identity keeps part positions and argument generation order.

use ai_interface::{ConversationMessage, Effort, ModelRequest};
use ai_models_core::test_support::{SseFixture, done_event, event, recording_streaming_client};
use serde_json::json;

use crate::{deepseek::DeepSeekModel, stream_test_support::observe};

#[tokio::test]
async fn delayed_tool_identity_keeps_the_first_part_before_later_content() {
    let (client, _) = recording_streaming_client(vec![SseFixture::Stream(vec![
        event(json!({"choices": [{"index": 0, "delta": {"tool_calls": [
            {"index": 0, "function": {"arguments": "{\"v\":"}}
        ]}}]})),
        event(
            json!({"choices": [{"index": 0, "delta": {"content": "checking", "tool_calls": [
                {"index": 1, "id": "call-1", "function": {"name": "second", "arguments": "{}"}}
            ]}}]}),
        ),
        event(json!({"choices": [{"index": 0, "delta": {"tool_calls": [
            {"index": 0, "id": "call-0", "function": {"name": "first", "arguments": "7}"}}
        ]}, "finish_reason": "tool_calls"}], "usage": {"prompt_tokens": 2, "completion_tokens": 3}})),
        done_event(),
    ])]);
    let model = DeepSeekModel::new(client, "key");
    let mut request = ModelRequest {
        system_prompt: String::new(),
        messages: vec![ConversationMessage::user("hello")],
        tools: Vec::new(),
        response_schema: None,
        controls: Default::default(),
    };
    request.controls.generation.effort = Some(Effort::Disabled);
    let mut events = Vec::new();
    let result = observe(&model, &request, &mut events).await;
    let diagnostic = format!("{result:?}");
    let ids = result.ok().map(|response| {
        response
            .tool_calls
            .into_iter()
            .map(|call| call.id)
            .collect::<Vec<_>>()
    });

    assert_eq!(
        ids,
        Some(vec!["call-0".to_owned(), "call-1".to_owned()]),
        "{diagnostic}"
    );
    assert_eq!(events.len(), 10);
    assert_eq!(
        events[..6],
        [
            json!({"ToolCallStart": {"part_index": 0, "id": "call-0", "name": "first"}}),
            json!({"ToolCallArgumentsDelta": {"part_index": 0, "delta": "{\"v\":"}}),
            json!({"TextDelta": {"part_index": 1, "delta": "checking"}}),
            json!({"ToolCallStart": {"part_index": 2, "id": "call-1", "name": "second"}}),
            json!({"ToolCallArgumentsDelta": {"part_index": 2, "delta": "{}"}}),
            json!({"ToolCallArgumentsDelta": {"part_index": 0, "delta": "7}"}}),
        ]
    );
}
