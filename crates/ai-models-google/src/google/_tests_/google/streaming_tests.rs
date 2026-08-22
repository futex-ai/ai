//! Google generate-content streaming and buffered-parity tests.

use std::time::Duration;

use ai_interface::{
    FinishReason, Model, ModelCallControls, ModelExecutionControls, StructuredOutputSchema,
};
use ai_models_core::{ThinkingLevel, synthetic_tool_call_scope};
use serde_json::{Value, json};

use super::{GoogleModel, simple_request, stream_support};
use crate::google::response::parse_response;

#[tokio::test]
async fn uses_streaming_endpoint_auth_and_completion_deadlines() {
    let events = vec![stream_support::event(text_chunk("Done", Some("STOP")))];
    let (http_client, requests) = stream_support::recording_streaming_client(vec![events]);
    let model = GoogleModel::new(http_client, "gemini-3.6-flash", "google-key");

    model
        .complete(&simple_request())
        .await
        .expect("streamed response should parse");

    let requests = requests.lock().expect("request lock should be available");
    let request = &requests[0];
    assert_eq!(
        request.url,
        "https://generativelanguage.googleapis.com/v1beta/models/gemini-3.6-flash:streamGenerateContent?alt=sse"
    );
    assert!(!request.url.contains(":generateContent"));
    assert_eq!(
        request.headers.get("x-goog-api-key"),
        Some(&"google-key".to_owned())
    );
    assert_eq!(request.timeout, Duration::from_secs(3_600));
    assert_eq!(request.idle_timeout, Some(Duration::from_secs(120)));
}

#[tokio::test]
async fn explicit_total_timeout_overrides_streaming_default() {
    let events = vec![stream_support::event(text_chunk("Done", Some("STOP")))];
    let (http_client, requests) = stream_support::recording_streaming_client(vec![events]);
    let model = GoogleModel::new(http_client, "gemini-3.6-flash", "key");
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
async fn merged_fragments_match_rich_buffered_response() {
    let request = simple_request();
    let buffered_body = rich_buffered_body();
    let chunks = vec![
        json!({
            "candidates": [{"content": {"parts": [
                {"text": "hidden ", "thought": true}
            ]}}],
            "usageMetadata": {"promptTokenCount": 1, "candidatesTokenCount": 1}
        }),
        json!({"candidates": [{"content": {"parts": [
            {"text": "reasoning", "thought": true}
        ]}}]}),
        text_chunk("Do", None),
        text_chunk("ne", Some("MAX_TOKENS")),
        json!({"candidates": [{"content": {"parts": [{
            "functionCall": {"name": "memory_read", "args": {"path": "root"}}
        }]}}]}),
        json!({
            "candidates": [{"finishReason": "STOP"}],
            "usageMetadata": buffered_body["usageMetadata"].clone()
        }),
    ];
    let events = chunks.into_iter().map(stream_support::event).collect();
    let (http_client, _) = stream_support::recording_streaming_client(vec![events]);
    let model = GoogleModel::new(http_client, "gemini-3.6-flash", "key");

    let streamed = model
        .complete(&request)
        .await
        .expect("streamed response should parse");
    let buffered = parse_response(
        "gemini-3.6-flash",
        "gemini-3.6-flash",
        ThinkingLevel::Disabled,
        &synthetic_tool_call_scope(&request),
        buffered_body,
        None,
    )
    .expect("buffered response should parse");

    assert_eq!(streamed, buffered);
    assert_eq!(streamed.finish_reason, FinishReason::ToolCalls);
}

#[tokio::test]
async fn structured_fragments_match_buffered_response() {
    let schema = StructuredOutputSchema {
        name: "status".to_owned(),
        schema: json!({
            "type": "object",
            "properties": {"summary": {"type": "string"}},
            "required": ["summary"]
        }),
    };
    let buffered_body = json!({
        "candidates": [{
            "finishReason": "STOP",
            "content": {"parts": [{"text": "{\"summary\":\"Done\"}"}]}
        }],
        "usageMetadata": {"promptTokenCount": 4, "candidatesTokenCount": 3}
    });
    let events = vec![
        stream_support::event(text_chunk("{\"summary\":", None)),
        stream_support::event(json!({
            "candidates": [{
                "finishReason": "STOP",
                "content": {"parts": [{"text": "\"Done\"}"}]}
            }],
            "usageMetadata": buffered_body["usageMetadata"].clone()
        })),
    ];
    let (http_client, _) = stream_support::recording_streaming_client(vec![events]);
    let model = GoogleModel::new(http_client, "gemini-3.6-flash", "key");
    let mut request = simple_request();
    request.response_schema = Some(schema.clone());

    let streamed = model
        .complete(&request)
        .await
        .expect("structured stream should parse");
    let buffered = parse_response(
        "gemini-3.6-flash",
        "gemini-3.6-flash",
        ThinkingLevel::Disabled,
        &synthetic_tool_call_scope(&request),
        buffered_body,
        Some(&schema),
    )
    .expect("buffered response should parse");

    assert_eq!(streamed, buffered);
}

#[tokio::test]
async fn prompt_block_stream_matches_buffered_response() {
    let body = json!({
        "promptFeedback": {"blockReason": "SAFETY"},
        "usageMetadata": {"promptTokenCount": 4, "totalTokenCount": 4}
    });
    let events = vec![stream_support::event(body.clone())];
    let (http_client, _) = stream_support::recording_streaming_client(vec![events]);
    let model = GoogleModel::new(http_client, "gemini-3.6-flash", "key");
    let request = simple_request();

    let streamed = model
        .complete(&request)
        .await
        .expect("prompt block should remain a filtered response");
    let buffered = parse_response(
        "gemini-3.6-flash",
        "gemini-3.6-flash",
        ThinkingLevel::Disabled,
        &synthetic_tool_call_scope(&request),
        body,
        None,
    )
    .expect("buffered prompt block should parse");

    assert_eq!(streamed, buffered);
}

#[tokio::test]
async fn same_chunk_text_parts_preserve_buffered_boundaries() {
    let body = json!({
        "candidates": [{
            "finishReason": "STOP",
            "content": {"parts": [{"text": "First"}, {"text": "Second"}]}
        }]
    });
    let events = vec![stream_support::event(body.clone())];
    let (http_client, _) = stream_support::recording_streaming_client(vec![events]);
    let model = GoogleModel::new(http_client, "gemini-3.6-flash", "key");
    let request = simple_request();

    let streamed = model
        .complete(&request)
        .await
        .expect("streamed text parts should parse");
    let buffered = parse_response(
        "gemini-3.6-flash",
        "gemini-3.6-flash",
        ThinkingLevel::Disabled,
        &synthetic_tool_call_scope(&request),
        body,
        None,
    )
    .expect("buffered text parts should parse");

    assert_eq!(streamed, buffered);
    assert_eq!(streamed.assistant_message, "First\nSecond");
}

fn text_chunk(text: &str, finish_reason: Option<&str>) -> Value {
    let mut candidate = json!({"content": {"parts": [{"text": text}]}});
    if let Some(finish_reason) = finish_reason {
        candidate["finishReason"] = json!(finish_reason);
    }
    json!({"candidates": [candidate]})
}

fn rich_buffered_body() -> Value {
    json!({
        "candidates": [{
            "finishReason": "STOP",
            "content": {"parts": [
                {"text": "hidden reasoning", "thought": true},
                {"text": "Done"},
                {"functionCall": {"name": "memory_read", "args": {"path": "root"}}}
            ]}
        }],
        "usageMetadata": {
            "promptTokenCount": 120,
            "candidatesTokenCount": 32,
            "cachedContentTokenCount": 40,
            "thoughtsTokenCount": 12,
            "totalTokenCount": 152
        }
    })
}
