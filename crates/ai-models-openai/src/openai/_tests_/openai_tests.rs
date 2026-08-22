//! Tests for OpenAI Responses request mapping and basic parsing.

use ai_interface::{
    ConversationMessage, FinishReason, Model, ModelRequest, StructuredOutputSchema, ToolDefinition,
};
use serde_json::json;

use super::OpenAiModel;
use crate::openai::stream_support::client_for_buffered_bodies;

#[tokio::test]
async fn builds_openai_tool_requests_and_parses_response() {
    let (http_client, requests) = client_for_buffered_bodies(vec![openai_tool_response()]);
    let model = OpenAiModel::new(http_client, "gpt-5.5", "sk-openai");

    let response = model
        .complete(&ModelRequest {
            system_prompt: "system".to_owned(),
            messages: vec![ConversationMessage::user("hello")],
            tools: vec![memory_read_tool()],
            response_schema: None,
            controls: Default::default(),
        })
        .await
        .expect("OpenAI response should parse");

    let requests = requests
        .lock()
        .expect("requests lock should not be poisoned");
    let body = requests[0].body.as_ref().expect("body present");
    assert_eq!(requests[0].url, "https://api.openai.com/v1/responses");
    assert_eq!(
        requests[0].headers.get("Authorization"),
        Some(&"Bearer sk-openai".to_owned())
    );
    assert_eq!(body["model"], "gpt-5.5");
    assert_eq!(body["instructions"], "system");
    assert_eq!(body["input"][0]["role"], "user");
    assert_eq!(body["input"][0]["content"], "hello");
    assert_eq!(body["tools"][0]["name"], "memory_read");
    assert_eq!(body["tools"][0]["type"], "function");
    assert_eq!(body["tools"][0]["strict"], false);
    assert_eq!(body["tool_choice"], "auto");
    assert_eq!(body["store"], false);
    assert!(body.get("messages").is_none());
    assert!(body.get("response_format").is_none());
    assert!(body.get("reasoning_effort").is_none());

    assert_eq!(response.assistant_message, "Done");
    assert_eq!(response.finish_reason, FinishReason::ToolCalls);
    assert_eq!(response.tool_calls.len(), 1);
    assert_eq!(response.tool_calls[0].id, "call_1");
    assert_eq!(response.tool_calls[0].name, "memory_read");
    assert_eq!(response.structured_output, None);
    assert_eq!(response.usage.total_tokens, 152);
}

#[tokio::test]
async fn builds_structured_output_requests_and_parses_response() {
    let (http_client, requests) = client_for_buffered_bodies(vec![openai_text_response(
        "{\"summary\":\"Done\",\"done\":true}",
    )]);
    let model = OpenAiModel::new(http_client, "gpt-5.5", "sk-openai");

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
        .expect("OpenAI structured response should parse");

    let requests = requests
        .lock()
        .expect("requests lock should not be poisoned");
    let format = &requests[0].body.as_ref().expect("body present")["text"]["format"];
    assert_eq!(format["type"], "json_schema");
    assert_eq!(format["name"], "status");
    assert_eq!(format["strict"], false);
    assert_eq!(
        response.structured_output,
        Some(json!({
            "summary": "Done",
            "done": true
        }))
    );
    assert_eq!(response.finish_reason, FinishReason::Stop);
}

fn memory_read_tool() -> ToolDefinition {
    ToolDefinition {
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
    }
}

fn openai_text_response(text: &str) -> serde_json::Value {
    json!({
        "status": "completed",
        "output": [{
            "type": "message",
            "role": "assistant",
            "content": [{ "type": "output_text", "text": text }]
        }],
        "usage": {
            "input_tokens": 120,
            "output_tokens": 32,
            "total_tokens": 152
        }
    })
}

fn openai_tool_response() -> serde_json::Value {
    json!({
        "status": "completed",
        "output": [
            {
                "type": "message",
                "role": "assistant",
                "content": [{ "type": "output_text", "text": "Done" }]
            },
            {
                "type": "function_call",
                "call_id": "call_1",
                "name": "memory_read",
                "arguments": "{\"path\":\"root\"}"
            }
        ],
        "usage": {
            "input_tokens": 120,
            "output_tokens": 32,
            "total_tokens": 152
        }
    })
}
