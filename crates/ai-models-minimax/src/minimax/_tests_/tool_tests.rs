//! MiniMax modern tool-call request and response tests.

use ai_interface::{
    ConversationMessage, ConversationRole, FinishReason, Model, ModelError, ModelRequest, ToolCall,
    ToolDefinition,
};
use json_http::JsonHttpResponse;
use serde_json::json;

use super::{MiniMaxModel, support::recording_http_client};

#[tokio::test]
async fn serializes_modern_tool_definitions_and_continuation_messages() {
    let (http_client, requests) = recording_http_client([tool_response()]);
    let model = MiniMaxModel::new(http_client, "MiniMax-M3", "minimax-key");
    let response = model
        .complete(&ModelRequest {
            system_prompt: "system".to_owned(),
            messages: vec![
                ConversationMessage::assistant(
                    "Checking",
                    vec![
                        ToolCall {
                            id: "call_1".to_owned(),
                            name: "memory_read".to_owned(),
                            input: json!({"path": "root"}),
                            operation_id: Some("runtime-operation".to_owned()),
                        },
                        ToolCall {
                            id: "call_2".to_owned(),
                            name: "clock_read".to_owned(),
                            input: json!({}),
                            operation_id: None,
                        },
                    ],
                ),
                ConversationMessage::tool("{\"value\":\"remembered\"}", "memory_read", "call_1"),
            ],
            tools: vec![ToolDefinition {
                name: "memory_read".to_owned(),
                description: "Read stored memory".to_owned(),
                input_schema: json!({
                    "type": "object",
                    "properties": {"path": {"type": "string"}},
                    "required": ["path"]
                }),
                activity_verb: Some("Remembering".to_owned()),
            }],
            response_schema: None,
        })
        .await
        .expect("MiniMax tool response should parse");

    let requests = requests
        .lock()
        .expect("requests lock should not be poisoned");
    let body = requests[0]
        .body
        .as_ref()
        .and_then(|body| body.as_json())
        .expect("JSON body should be present");
    assert_eq!(
        body["tools"],
        json!([{
            "type": "function",
            "function": {
                "name": "memory_read",
                "description": "Read stored memory",
                "parameters": {
                    "type": "object",
                    "properties": {"path": {"type": "string"}},
                    "required": ["path"]
                }
            }
        }])
    );
    assert_eq!(body["tool_choice"], "auto");
    assert_eq!(
        body["messages"][1],
        json!({
            "role": "assistant",
            "content": "Checking",
            "tool_calls": [
                {
                    "id": "call_1",
                    "type": "function",
                    "function": {
                        "name": "memory_read",
                        "arguments": "{\"path\":\"root\"}"
                    }
                },
                {
                    "id": "call_2",
                    "type": "function",
                    "function": {
                        "name": "clock_read",
                        "arguments": "{}"
                    }
                }
            ]
        })
    );
    assert_eq!(
        body["messages"][2],
        json!({
            "role": "tool",
            "content": "{\"value\":\"remembered\"}",
            "tool_call_id": "call_1"
        })
    );
    assert!(body["messages"][2].get("name").is_none());
    assert!(!body.to_string().contains("function_call"));

    assert_eq!(response.finish_reason, FinishReason::ToolCalls);
    assert_eq!(response.assistant_message, "I need both results.");
    assert_eq!(response.tool_calls.len(), 2);
    assert_eq!(response.tool_calls[0].id, "call_a");
    assert_eq!(response.tool_calls[0].name, "memory_read");
    assert_eq!(response.tool_calls[0].input, json!({"path": "root"}));
    assert_eq!(response.tool_calls[0].operation_id, None);
    assert_eq!(response.tool_calls[1].id, "call_b");
    assert_eq!(response.tool_calls[1].name, "clock_read");
    assert_eq!(response.tool_calls[1].input, json!({}));
    assert_eq!(response.tool_calls[1].operation_id, None);
}

#[tokio::test]
async fn omits_unavailable_assistant_content() {
    let (http_client, requests) = recording_http_client([tool_response()]);
    MiniMaxModel::new(http_client, "MiniMax-M3", "minimax-key")
        .complete(&ModelRequest {
            system_prompt: "system".to_owned(),
            messages: vec![ConversationMessage::assistant(
                "",
                vec![ToolCall {
                    id: "call_1".to_owned(),
                    name: "memory_read".to_owned(),
                    input: json!({"path": "root"}),
                    operation_id: None,
                }],
            )],
            tools: Vec::new(),
            response_schema: None,
        })
        .await
        .expect("MiniMax response should parse");

    let requests = requests
        .lock()
        .expect("requests lock should not be poisoned");
    let assistant_message = &requests[0]
        .body
        .as_ref()
        .and_then(|body| body.as_json())
        .expect("JSON body should be present")["messages"][1];
    assert!(assistant_message.get("content").is_none());
    assert_eq!(assistant_message["tool_calls"][0]["id"], "call_1");
}

#[tokio::test]
async fn preserves_empty_tool_result_content() {
    let (http_client, requests) = recording_http_client([tool_response()]);
    MiniMaxModel::new(http_client, "MiniMax-M3", "minimax-key")
        .complete(&ModelRequest {
            system_prompt: "system".to_owned(),
            messages: vec![ConversationMessage::tool("", "memory_read", "call_empty")],
            tools: Vec::new(),
            response_schema: None,
        })
        .await
        .expect("MiniMax response should parse");

    let requests = requests
        .lock()
        .expect("requests lock should not be poisoned");
    let tool_message = &requests[0]
        .body
        .as_ref()
        .and_then(|body| body.as_json())
        .expect("JSON body should be present")["messages"][1];
    assert_eq!(
        tool_message,
        &json!({
            "role": "tool",
            "content": "",
            "tool_call_id": "call_empty"
        })
    );
}

#[tokio::test]
async fn rejects_malformed_tool_arguments() {
    let (http_client, _) = recording_http_client([JsonHttpResponse {
        status: 200,
        body: json!({
            "choices": [{
                "finish_reason": "tool_calls",
                "message": {
                    "content": "",
                    "tool_calls": [{
                        "id": "call_bad",
                        "type": "function",
                        "function": {
                            "name": "memory_read",
                            "arguments": "{bad"
                        }
                    }]
                }
            }]
        }),
    }]);
    let error = MiniMaxModel::new(http_client, "MiniMax-M3", "minimax-key")
        .complete(&simple_request())
        .await
        .expect_err("malformed tool arguments should fail");

    assert!(matches!(error, ModelError::Provider { .. }));
    assert!(
        error
            .to_string()
            .contains("invalid tool call JSON arguments")
    );
}

fn simple_request() -> ModelRequest {
    ModelRequest {
        system_prompt: "system".to_owned(),
        messages: vec![ConversationMessage {
            role: ConversationRole::User,
            content: "Use tools".to_owned(),
            content_parts: Vec::new(),
            name: None,
            tool_call_id: None,
            tool_calls: Vec::new(),
            provider_context: Vec::new(),
        }],
        tools: Vec::new(),
        response_schema: None,
    }
}

fn tool_response() -> JsonHttpResponse<serde_json::Value> {
    JsonHttpResponse {
        status: 200,
        body: json!({
            "choices": [{
                "finish_reason": "tool_calls",
                "message": {
                    "content": "I need both results.",
                    "tool_calls": [
                        {
                            "id": "call_a",
                            "type": "function",
                            "function": {
                                "name": "memory_read",
                                "arguments": "{\"path\":\"root\"}"
                            }
                        },
                        {
                            "id": "call_b",
                            "type": "function",
                            "function": {
                                "name": "clock_read",
                                "arguments": "{}"
                            }
                        }
                    ]
                }
            }]
        }),
    }
}
