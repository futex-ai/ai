//! Google generate-content streaming and buffered-parity tests.

use std::time::Duration;

use ai_interface::{
    ConversationPart, FinishReason, Model, ModelCallControls, ModelExecutionControls, ModelUsage,
    StructuredOutputSchema, ToolCall,
};
use serde_json::{Value, json};

use super::stream_support::{custom_spec, expected_response};
use super::{GoogleModel, simple_request, stream_support};

#[tokio::test]
async fn uses_streaming_endpoint_auth_and_completion_deadlines() {
    let events = vec![stream_support::event(text_chunk("Done", Some("STOP")))];
    let (http_client, requests) = stream_support::recording_streaming_client(vec![events]);
    let model = GoogleModel::new(http_client, custom_spec("gemini-3.6-flash"), "google-key");

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
    let model = GoogleModel::new(http_client, custom_spec("gemini-3.6-flash"), "key");
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
    let model = GoogleModel::new(http_client, custom_spec("gemini-3.6-flash"), "key");

    let streamed = model
        .complete(&request)
        .await
        .expect("streamed response should parse");
    let mut buffered = expected_response(
        "Done",
        FinishReason::ToolCalls,
        ModelUsage {
            input_tokens: 80,
            output_tokens: 32,
            cached_input_tokens: 40,
            reasoning_tokens: 12,
            total_tokens: 152,
            ..Default::default()
        },
    );
    buffered
        .parts
        .insert(0, ConversationPart::reasoning("hidden reasoning"));
    buffered
        .parts
        .push(ConversationPart::from_tool_call(ToolCall {
            id: "google_function_call:2bb0037dd4a68f16".to_owned(),
            name: "memory_read".to_owned(),
            input: json!({"path": "root"}),
            operation_id: Some("google_function_call:2bb0037dd4a68f16".to_owned()),
        }));
    *buffered.parts.last_mut().unwrap().provider_state_mut() = Some(
        serde_json::from_value(json!({
            "provider": "google", "model": "gemini-3.6-flash", "version": 1,
            "payload": {
                "scope": "part",
                "parts": [{"functionCall": {"name": "memory_read", "args": {"path": "root"}}}],
                "unsigned_call": {
                    "id": "google_function_call:2bb0037dd4a68f16", "name": "memory_read",
                    "args": {"path": "root"}
                }
            }
        }))
        .unwrap(),
    );

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
    let model = GoogleModel::new(http_client, custom_spec("gemini-3.6-flash"), "key");
    let mut request = simple_request();
    request.response_schema = Some(schema.clone());

    let streamed = model
        .complete(&request)
        .await
        .expect("structured stream should parse");
    let mut buffered = expected_response(
        "{\"summary\":\"Done\"}",
        FinishReason::Stop,
        ModelUsage {
            input_tokens: 4,
            output_tokens: 3,
            total_tokens: 7,
            ..Default::default()
        },
    );
    buffered.structured_output = Some(json!({"summary": "Done"}));

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
    let model = GoogleModel::new(http_client, custom_spec("gemini-3.6-flash"), "key");
    let request = simple_request();

    let streamed = model
        .complete(&request)
        .await
        .expect("prompt block should remain a filtered response");
    let buffered = expected_response(
        "",
        FinishReason::Filtered,
        ModelUsage {
            input_tokens: 4,
            total_tokens: 4,
            ..Default::default()
        },
    );

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
    let model = GoogleModel::new(http_client, custom_spec("gemini-3.6-flash"), "key");
    let request = simple_request();

    let streamed = model
        .complete(&request)
        .await
        .expect("streamed text parts should parse");
    let mut buffered =
        expected_response("First\nSecond", FinishReason::Stop, ModelUsage::default());
    buffered.parts = vec![
        ConversationPart::text("First").with_provider_state(
            serde_json::from_value(json!({
                "provider": "google", "model": "gemini-3.6-flash", "version": 1,
                "payload": {"scope": "normalized_text", "parts": []}
            }))
            .unwrap(),
        ),
        ConversationPart::text("\nSecond"),
    ];

    assert_eq!(streamed, buffered);
    assert_eq!(streamed.text(), "First\nSecond");
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
