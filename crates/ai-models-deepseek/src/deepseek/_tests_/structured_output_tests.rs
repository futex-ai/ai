//! DeepSeek locally validated JSON-object response tests.

use ai_interface::{Model, ModelError, ModelRequest, ModelResponse, StructuredOutputSchema};
use json_http::JsonHttpResponse;
use serde_json::{Value, json};

use super::{DeepSeekModel, test_support::recording_http_client};

#[tokio::test]
async fn prompts_for_json_object_and_validates_the_requested_schema_locally() {
    let schema = status_schema();
    let (http_client, requests) = recording_http_client(JsonHttpResponse {
        status: 200,
        body: stopped_response(
            json!({
                "summary": "Done",
                "done": true
            })
            .to_string(),
        ),
    });
    let response = DeepSeekModel::new(http_client, "deepseek-key")
        .complete(&request_with_schema(schema.clone()))
        .await
        .expect("valid structured response should parse");

    let requests = requests
        .lock()
        .expect("requests lock should not be poisoned");
    let body = requests[0]
        .body
        .as_ref()
        .and_then(|body| body.as_json())
        .expect("JSON body should be present");
    let system_prompt = body["messages"][0]["content"]
        .as_str()
        .expect("system prompt should be text");

    assert!(system_prompt.starts_with("system"));
    assert!(system_prompt.contains("JSON"));
    assert!(system_prompt.contains("raw JSON"));
    assert!(system_prompt.contains("Do not use Markdown"));
    assert!(system_prompt.contains("status"));
    assert!(system_prompt.contains(&schema.schema.to_string()));
    assert_eq!(body["response_format"], json!({"type": "json_object"}));
    assert_eq!(
        response.structured_output,
        Some(json!({"summary": "Done", "done": true}))
    );
}

#[tokio::test]
async fn rejects_empty_invalid_mismatched_and_invalid_schema_output() {
    let cases = [
        (String::new(), status_schema()),
        ("not json".to_owned(), status_schema()),
        (
            json!({"summary": 7, "done": true}).to_string(),
            status_schema(),
        ),
        (
            json!({"summary": "Done"}).to_string(),
            StructuredOutputSchema {
                name: "invalid".to_owned(),
                schema: json!({"type": 7}),
            },
        ),
    ];

    for (content, schema) in cases {
        let error = complete_with_schema(stopped_response(content), schema)
            .await
            .expect_err("invalid structured output should fail");
        assert!(matches!(error, ModelError::Provider { .. }));
    }
}

#[tokio::test]
async fn parses_structured_output_only_for_natural_stops_without_calls() {
    for finish_reason in [
        Some("tool_calls"),
        Some("length"),
        Some("content_filter"),
        Some("future_reason"),
        None,
    ] {
        let mut choice = json!({
            "message": {
                "content": "not json",
                "reasoning_content": "private",
                "tool_calls": [{
                    "id": "call_1",
                    "type": "function",
                    "function": {
                        "name": "memory_read",
                        "arguments": "{}"
                    }
                }]
            }
        });
        if let Some(finish_reason) = finish_reason {
            choice["finish_reason"] = json!(finish_reason);
        }
        let response = complete_with_schema(json!({"choices": [choice]}), status_schema())
            .await
            .expect("non-stop response should skip structured parsing");

        assert_eq!(response.structured_output, None);
    }
}

async fn complete_with_schema(
    body: Value,
    schema: StructuredOutputSchema,
) -> std::result::Result<ModelResponse, ModelError> {
    let (http_client, _) = recording_http_client(JsonHttpResponse { status: 200, body });
    DeepSeekModel::new(http_client, "deepseek-key")
        .complete(&request_with_schema(schema))
        .await
}

fn request_with_schema(schema: StructuredOutputSchema) -> ModelRequest {
    ModelRequest {
        system_prompt: "system".to_owned(),
        messages: Vec::new(),
        tools: Vec::new(),
        response_schema: Some(schema),
        controls: Default::default(),
    }
}

fn status_schema() -> StructuredOutputSchema {
    StructuredOutputSchema {
        name: "status".to_owned(),
        schema: json!({
            "type": "object",
            "properties": {
                "summary": {"type": "string"},
                "done": {"type": "boolean"}
            },
            "required": ["summary", "done"]
        }),
    }
}

fn stopped_response(content: String) -> Value {
    json!({
        "choices": [{
            "finish_reason": "stop",
            "message": {
                "content": content,
                "reasoning_content": "private"
            }
        }]
    })
}
