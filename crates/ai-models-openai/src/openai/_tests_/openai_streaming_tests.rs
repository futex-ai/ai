//! OpenAI Responses streaming and buffered-parity tests.

use std::sync::Arc;
use std::time::Duration;

use ai_interface::{
    ConversationMessage, FinishReason, Model, ModelCallControls, ModelExecutionControls,
    ModelRequest, StructuredOutputSchema,
};
use ai_models_core::ThinkingLevel;
use json_http::StaticHeaderAuth;
use serde_json::{Value, json};

use super::{OpenAiModel, response::parse_response};
use crate::openai::stream_support::{event, recording_streaming_client, terminal_event};

#[tokio::test]
async fn sends_streaming_request_with_completion_deadlines() {
    let body = text_body("Done");
    let events = vec![terminal_event("response.completed", body)];
    let (http_client, requests) = recording_streaming_client(vec![events]);
    let model = OpenAiModel::new(http_client, "gpt-5.5", "key");

    model
        .complete(&simple_request())
        .await
        .expect("streamed response should parse");

    let requests = requests.lock().expect("request lock should be available");
    let request = &requests[0];
    assert_eq!(request.body.as_ref().expect("request body")["stream"], true);
    assert_eq!(request.timeout, Duration::from_secs(3_600));
    assert_eq!(request.idle_timeout, Some(Duration::from_secs(120)));
}

#[tokio::test]
async fn explicit_total_timeout_overrides_streaming_default() {
    let events = vec![terminal_event("response.completed", text_body("Done"))];
    let (http_client, requests) = recording_streaming_client(vec![events]);
    let model = OpenAiModel::new(http_client, "gpt-5.5", "key");
    let mut request = simple_request();
    request.controls = ModelCallControls {
        execution: ModelExecutionControls {
            total_timeout: Some(Duration::from_secs(75)),
            ..Default::default()
        },
        ..Default::default()
    };

    model
        .complete(&request)
        .await
        .expect("streamed response should parse");

    let requests = requests.lock().expect("request lock should be available");
    assert_eq!(requests[0].timeout, Duration::from_secs(75));
    assert_eq!(requests[0].idle_timeout, Some(Duration::from_secs(120)));
}

#[tokio::test]
async fn completed_stream_matches_rich_buffered_response() {
    let body = rich_tool_body();
    let events = vec![
        event(
            "response.created",
            json!({"type": "response.created", "response": {"status": "in_progress"}}),
        ),
        event(
            "response.output_text.delta",
            json!({"type": "response.output_text.delta", "delta": "Done"}),
        ),
        event("future.event", json!({"type": "future.event"})),
        terminal_event("response.completed", body.clone()),
    ];
    let (http_client, _) = recording_streaming_client(vec![events]);
    let model = OpenAiModel::with_catalog_auth(
        http_client,
        "gpt-5.5-thinking-high",
        "gpt-5.5",
        ThinkingLevel::High,
        Arc::new(StaticHeaderAuth::default()),
    );

    let streamed = model
        .complete(&simple_request())
        .await
        .expect("streamed response should parse");
    let buffered = parse_response(
        "gpt-5.5-thinking-high",
        "gpt-5.5",
        ThinkingLevel::High,
        body,
        None,
    )
    .expect("buffered response should parse");

    assert_eq!(streamed, buffered);
    assert_eq!(streamed.finish_reason, FinishReason::ToolCalls);
}

#[tokio::test]
async fn completed_structured_stream_matches_buffered_response() {
    let body = text_body("{\"summary\":\"Done\"}");
    let schema = StructuredOutputSchema {
        name: "status".to_owned(),
        schema: json!({
            "type": "object",
            "properties": {"summary": {"type": "string"}},
            "required": ["summary"]
        }),
    };
    let events = vec![terminal_event("response.completed", body.clone())];
    let (http_client, _) = recording_streaming_client(vec![events]);
    let model = OpenAiModel::new(http_client, "gpt-5.5", "key");
    let mut request = simple_request();
    request.response_schema = Some(schema.clone());

    let streamed = model
        .complete(&request)
        .await
        .expect("structured stream should parse");
    let buffered = parse_response(
        "gpt-5.5",
        "gpt-5.5",
        ThinkingLevel::Disabled,
        body,
        Some(&schema),
    )
    .expect("buffered response should parse");

    assert_eq!(streamed, buffered);
}

#[tokio::test]
async fn incomplete_stream_matches_buffered_truncation() {
    let body = json!({
        "status": "incomplete",
        "incomplete_details": {"reason": "max_output_tokens"},
        "output": [{
            "type": "message",
            "content": [{"type": "output_text", "text": "Partial"}]
        }],
        "usage": {"input_tokens": 4, "output_tokens": 8, "total_tokens": 12}
    });
    let events = vec![terminal_event("response.incomplete", body.clone())];
    let (http_client, _) = recording_streaming_client(vec![events]);
    let model = OpenAiModel::new(http_client, "gpt-5.5", "key");

    let streamed = model
        .complete(&simple_request())
        .await
        .expect("incomplete response should remain usable");
    let buffered = parse_response("gpt-5.5", "gpt-5.5", ThinkingLevel::Disabled, body, None)
        .expect("buffered incomplete response should parse");

    assert_eq!(streamed, buffered);
    assert_eq!(streamed.finish_reason, FinishReason::Truncated);
}

fn simple_request() -> ModelRequest {
    ModelRequest {
        system_prompt: "system".to_owned(),
        messages: vec![ConversationMessage::user("hello")],
        tools: Vec::new(),
        response_schema: None,
        controls: Default::default(),
    }
}

fn text_body(text: &str) -> Value {
    json!({
        "status": "completed",
        "output": [{
            "type": "message",
            "content": [{"type": "output_text", "text": text}]
        }],
        "usage": {"input_tokens": 4, "output_tokens": 2, "total_tokens": 6}
    })
}

fn rich_tool_body() -> Value {
    json!({
        "status": "completed",
        "output": [
            {
                "type": "reasoning",
                "id": "rs_1",
                "summary": [{"type": "summary_text", "text": "Use memory"}],
                "encrypted_content": "encrypted"
            },
            {
                "type": "message",
                "phase": "commentary",
                "content": [{"type": "output_text", "text": "Checking. "}]
            },
            {"type": "web_search_call", "id": "ws_1", "status": "completed"},
            {
                "type": "function_call",
                "id": "fc_1",
                "call_id": "call_1",
                "name": "memory_read",
                "arguments": "{\"path\":\"root\"}"
            }
        ],
        "usage": {
            "input_tokens": 120,
            "input_tokens_details": {"cached_tokens": 40},
            "output_tokens": 32,
            "output_tokens_details": {"reasoning_tokens": 12},
            "total_tokens": 152
        }
    })
}
