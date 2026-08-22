use ai_interface::{
    ConversationMessage, ConversationRole, FinishReason, Model, ModelRequest,
    StructuredOutputSchema, ToolDefinition,
};
use json_http::JsonHttpResponse;
use serde_json::json;

use super::{GoogleModel, recording_http_client, simple_request};

#[tokio::test]
async fn builds_google_tool_requests_and_parses_response() {
    let (http_client, requests) = recording_http_client(JsonHttpResponse {
        status: 200,
        body: json!({
            "candidates": [{
                "finishReason": "STOP",
                "content": {
                    "parts": [
                        { "text": "Done" },
                        {
                            "functionCall": {
                                "id": "call_1",
                                "name": "memory_read",
                                "args": { "path": "root" }
                            }
                        }
                    ]
                }
            }],
            "usageMetadata": {
                "promptTokenCount": 120,
                "candidatesTokenCount": 32,
                "totalTokenCount": 152
            }
        }),
    });
    let model = GoogleModel::new(http_client, "gemini-2.5-pro", "google-key");

    let response = model
        .complete(&ModelRequest {
            system_prompt: "system".to_owned(),
            messages: vec![ConversationMessage {
                role: ConversationRole::User,
                content: "hello".to_owned(),
                content_parts: Vec::new(),
                name: None,
                tool_call_id: None,
                tool_calls: Vec::new(),
                provider_context: Vec::new(),
            }],
            tools: vec![ToolDefinition {
                name: "memory_read".to_owned(),
                description: "Read memory".to_owned(),
                input_schema: json!({
                    "type": "object",
                    "properties": {
                        "path": {"type": "string"}
                    },
                    "required": ["path"]
                }),
                activity_verb: Some("Remembering".to_owned()),
            }],
            response_schema: None,
            controls: Default::default(),
        })
        .await
        .expect("Google response should parse");

    let requests = requests
        .lock()
        .expect("requests lock should not be poisoned");
    assert_eq!(
        requests[0].url,
        "https://generativelanguage.googleapis.com/v1beta/models/gemini-2.5-pro:streamGenerateContent?alt=sse"
    );
    assert_eq!(
        requests[0].headers.get("x-goog-api-key"),
        Some(&"google-key".to_owned())
    );
    assert_eq!(
        requests[0].body.as_ref().expect("body present")["tools"][0]["functionDeclarations"][0]["name"],
        "memory_read"
    );

    assert_eq!(response.assistant_message, "Done");
    assert_eq!(response.finish_reason, FinishReason::ToolCalls);
    assert_eq!(response.tool_calls.len(), 1);
    assert_eq!(response.tool_calls[0].name, "memory_read");
    assert_eq!(response.structured_output, None);
    assert_eq!(response.usage.total_tokens, 152);
}

#[tokio::test]
async fn google_function_call_without_args_uses_empty_object_input() {
    let (http_client, _) = recording_http_client(JsonHttpResponse {
        status: 200,
        body: json!({
            "candidates": [{
                "finishReason": "STOP",
                "content": {
                    "parts": [{
                        "functionCall": {
                            "id": "call_1",
                            "name": "status_check"
                        }
                    }]
                }
            }]
        }),
    });
    let model = GoogleModel::new(http_client, "gemini-2.5-pro", "google-key");

    let response = model
        .complete(&simple_request())
        .await
        .expect("Google no-arg function call should parse");

    assert_eq!(response.finish_reason, FinishReason::ToolCalls);
    assert_eq!(response.tool_calls.len(), 1);
    assert_eq!(response.tool_calls[0].input, json!({}));
}

#[tokio::test]
async fn builds_google_structured_output_requests_and_parses_response() {
    let (http_client, requests) = recording_http_client(JsonHttpResponse {
        status: 200,
        body: json!({
            "candidates": [{
                "finishReason": "STOP",
                "content": {
                    "parts": [
                        { "text": "{\"summary\":\"Done\",\"done\":true}" }
                    ]
                }
            }],
            "usageMetadata": {
                "promptTokenCount": 12,
                "candidatesTokenCount": 6,
                "cachedContentTokenCount": 4,
                "thoughtsTokenCount": 5,
                "totalTokenCount": 23
            }
        }),
    });
    let model = GoogleModel::new(http_client, "gemini-2.5-pro", "google-key");

    let response = model
        .complete(&ModelRequest {
            system_prompt: "system".to_owned(),
            messages: vec![ConversationMessage::user("hello")],
            tools: Vec::new(),
            response_schema: Some(StructuredOutputSchema {
                name: "status".to_owned(),
                schema: json!({
                    "type": "object",
                    "properties": {
                        "summary": {"type": "string"},
                        "done": {"type": "boolean"}
                    },
                    "required": ["summary", "done"]
                }),
            }),
            controls: Default::default(),
        })
        .await
        .expect("Google structured response should parse");

    let requests = requests
        .lock()
        .expect("requests lock should not be poisoned");
    assert_eq!(
        requests[0].body.as_ref().expect("body present")["generationConfig"]["responseMimeType"],
        "application/json"
    );
    assert_eq!(
        response.structured_output,
        Some(json!({
            "summary": "Done",
            "done": true
        }))
    );
    assert_eq!(response.finish_reason, FinishReason::Stop);
}

#[tokio::test]
async fn missing_google_total_tokens_falls_back_to_normalized_usage_sum() {
    let (http_client, _) = recording_http_client(JsonHttpResponse {
        status: 200,
        body: json!({
            "candidates": [{
                "finishReason": "STOP",
                "content": {
                    "parts": [{ "text": "Done" }]
                }
            }],
            "usageMetadata": {
                "promptTokenCount": 12,
                "candidatesTokenCount": 6,
                "cachedContentTokenCount": 4,
                "thoughtsTokenCount": 5
            }
        }),
    });
    let model = GoogleModel::new(http_client, "gemini-2.5-pro", "google-key");

    let response = model
        .complete(&simple_request())
        .await
        .expect("Google response should parse");

    assert_eq!(response.usage.input_tokens, 8);
    assert_eq!(response.usage.output_tokens, 6);
    assert_eq!(response.usage.cached_input_tokens, 4);
    assert_eq!(response.usage.reasoning_tokens, 5);
    assert_eq!(response.usage.total_tokens, 23);
}
