//! OpenAI Responses streaming and buffered-parity tests.

use std::sync::Arc;
use std::time::Duration;

use ai_interface::{
    ConversationMessage, ConversationPart, Effort, FinishReason, Model, ModelCallControls,
    ModelExecutionControls, ModelRequest, ModelUsage, StructuredOutputSchema, ToolCall,
};
use json_http::StaticHeaderAuth;
use serde_json::{Value, json};

use crate::GPT_5_5;
use crate::openai::stream_support::custom_spec;
use crate::openai::stream_support::{event, recording_streaming_client, terminal_event};

use super::{OpenAiModel, stream_support::expected_response};
use crate::openai::stream_support::state;

#[tokio::test]
async fn sends_streaming_request_with_completion_deadlines() {
    let body = text_body("Done");
    let events = vec![terminal_event("response.completed", body)];
    let (http_client, requests) = recording_streaming_client(vec![events]);
    let model = OpenAiModel::new(http_client, custom_spec("gpt-5.5"), "key");

    model
        .complete(&simple_request())
        .await
        .expect("streamed response should parse");

    let requests = requests.lock().expect("request lock should be available");
    let request = &requests[0];
    assert_eq!(
        serde_json::from_slice::<serde_json::Value>(
            request
                .body
                .as_ref()
                .expect("request body")
                .as_json_bytes()
                .expect("JSON request body")
        )
        .expect("valid JSON request body")["stream"],
        true
    );
    assert_eq!(request.timeout, Duration::from_secs(3_600));
    assert_eq!(request.idle_timeout, Some(Duration::from_secs(120)));
}

#[tokio::test]
async fn explicit_total_timeout_overrides_streaming_default() {
    let events = vec![terminal_event("response.completed", text_body("Done"))];
    let (http_client, requests) = recording_streaming_client(vec![events]);
    let model = OpenAiModel::new(http_client, custom_spec("gpt-5.5"), "key");
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
            json!({"type": "response.output_text.delta", "delta": "Checking. "}),
        ),
        event("future.event", json!({"type": "future.event"})),
        terminal_event("response.completed", body.clone()),
    ];
    let (http_client, _) = recording_streaming_client(vec![events]);
    let model =
        OpenAiModel::with_catalog_auth(http_client, GPT_5_5, Arc::new(StaticHeaderAuth::default()));

    let mut request = simple_request();
    request.controls.generation.effort = Some(Effort::High);
    let streamed = model
        .complete(&request)
        .await
        .expect("streamed response should parse");
    let mut buffered = expected_response(
        "Checking. ",
        Effort::High,
        FinishReason::ToolCalls,
        ModelUsage {
            input_tokens: 80,
            output_tokens: 20,
            cached_input_tokens: 40,
            reasoning_tokens: 12,
            total_tokens: 152,
            ..Default::default()
        },
    );
    let tool_calls = [ToolCall {
        id: "call_1".to_owned(),
        name: "memory_read".to_owned(),
        input: json!({"path": "root"}),
        operation_id: None,
    }];
    buffered.parts = vec![
        ConversationPart::reasoning("Use memory").with_provider_state(state(json!({"id":"rs_1","summary":[{"type":"summary_text","text":"Use memory"}],"encrypted_content":"encrypted"}))),
        ConversationPart::text("Checking. ").with_provider_state(state(json!({"phase":"commentary"}))),
        ConversationPart::from_tool_call(tool_calls[0].clone()).with_provider_state(state(json!({"id":"fc_1","call_id":"call_1","name":"memory_read","arguments":"{\"path\":\"root\"}"}))),
    ];

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
    let model = OpenAiModel::new(http_client, custom_spec("gpt-5.5"), "key");
    let mut request = simple_request();
    request.response_schema = Some(schema.clone());

    let streamed = model
        .complete(&request)
        .await
        .expect("structured stream should parse");
    let mut buffered = expected_response(
        "{\"summary\":\"Done\"}",
        Effort::Disabled,
        FinishReason::Stop,
        ModelUsage {
            input_tokens: 4,
            output_tokens: 2,
            total_tokens: 6,
            ..Default::default()
        },
    );
    buffered.structured_output = Some(json!({"summary": "Done"}));

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
    let model = OpenAiModel::new(http_client, custom_spec("gpt-5.5"), "key");

    let streamed = model
        .complete(&simple_request())
        .await
        .expect("incomplete response should remain usable");
    let buffered = expected_response(
        "Partial",
        Effort::Disabled,
        FinishReason::Truncated,
        ModelUsage {
            input_tokens: 4,
            output_tokens: 8,
            total_tokens: 12,
            ..Default::default()
        },
    );

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
