//! Explicit provider termination can report an absent finish reason.

use ai_interface::ErrorKind;

use ai_interface::{ConversationMessage, FinishReason, ModelRequest};
use ai_models_core::test_support::{SseFixture, done_event, event, recording_streaming_client};
use serde_json::json;

use crate::{
    deepseek::DeepSeekModel,
    minimax::{MINIMAX_M3, MiniMaxModel},
    stream_test_support::observe,
    xai::{GROK_4_5, XaiModel},
};

#[tokio::test]
async fn maps_missing_finish_after_done_and_keeps_clean_eof_strict() {
    let chunk =
        || event(json!({"choices": [{"index": 0, "delta": {"content": "answer"}}], "usage": {}}));
    let request = ModelRequest {
        system_prompt: String::new(),
        messages: vec![ConversationMessage::user("hello")],
        tools: Vec::new(),
        response_schema: None,
        controls: Default::default(),
    };
    let (client, _) =
        recording_streaming_client(vec![SseFixture::Stream(vec![chunk(), done_event()])]);
    let model = DeepSeekModel::new(client, "key");
    let mut events = Vec::new();
    let result = observe(&model, &request, &mut events).await;
    let diagnostic = format!("{result:?}");

    assert_eq!(
        result.ok().map(|response| response.finish_reason),
        Some(FinishReason::Other("missing".to_owned())),
        "{diagnostic}"
    );
    assert_eq!(events.len(), 3);
    let (client, _) = recording_streaming_client(vec![SseFixture::Stream(vec![chunk()])]);
    let model = MiniMaxModel::new(client, MINIMAX_M3, "key");
    let mut events = Vec::new();
    let result = observe(&model, &request, &mut events).await;
    assert!(matches!(result, Err(ref error) if error.core().kind == ErrorKind::Interrupted));
    assert_eq!(events.len(), 1);
}

#[tokio::test]
async fn tool_finish_without_a_call_fails_before_usage() {
    for chunk in [
        json!({"choices": [{"index": 0, "delta": {}, "finish_reason": "tool_calls"}], "usage": {}}),
        json!({"choices": [{"index": 0, "delta": {"tool_calls":[{"index":0,"function":{"name":"lookup","arguments":"{}"}}]}, "finish_reason": "tool_calls"}], "usage": {}}),
        json!({"choices": [{"index": 0, "delta": {"tool_calls":[{"index":0,"id":"call_1","function":{"arguments":"{}"}}]}, "finish_reason": "tool_calls"}], "usage": {}}),
        json!({"choices": [{"index": 0, "delta": {"function_call":{"arguments":"{}"}}, "finish_reason": "function_call"}], "usage": {}}),
    ] {
        let (client, _) =
            recording_streaming_client(vec![SseFixture::Stream(vec![event(chunk), done_event()])]);
        let request = ModelRequest {
            system_prompt: String::new(),
            messages: vec![ConversationMessage::user("hello")],
            tools: Vec::new(),
            response_schema: None,
            controls: Default::default(),
        };
        let model = XaiModel::new(client, GROK_4_5, "key");
        let mut events = Vec::new();
        let result = observe(&model, &request, &mut events).await;

        assert!(matches!(result, Err(ref error) if error.core().kind == ErrorKind::InvalidOutput));
        assert_eq!(events.len(), 0);
    }
}
