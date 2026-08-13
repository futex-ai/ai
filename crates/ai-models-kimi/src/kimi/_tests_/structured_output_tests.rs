//! Kimi structured-output request and response tests.

use ai_interface::{FinishReason, ModelError, StructuredOutputSchema};
use ai_models_core::ThinkingLevel;
use serde_json::json;

use crate::KIMI_K3;

use super::{
    client::KimiReasoningEffort, request::build_request, response::parse_response,
    test_support::simple_request,
};

#[test]
fn sends_actual_schema_with_non_strict_response_format() {
    let mut request = simple_request();
    request.response_schema = Some(status_schema());
    let body = build_request(KIMI_K3, KimiReasoningEffort::Max, &request)
        .expect("Kimi request should build");
    let body = serde_json::to_value(body).expect("Kimi request should serialize");

    assert_eq!(body["response_format"]["type"], "json_schema");
    assert_eq!(
        body["response_format"]["json_schema"]["name"],
        "status_response"
    );
    assert_eq!(
        body["response_format"]["json_schema"]["schema"],
        status_schema().schema
    );
    assert_eq!(
        body["response_format"]["json_schema"]["strict"],
        json!(false)
    );
}

#[test]
fn parses_and_locally_validates_structured_output() {
    let response = structured_response(
        "stop",
        Some("{\"summary\":\"Done\",\"done\":true}"),
        &status_schema(),
    )
    .expect("valid structured output should parse");

    assert_eq!(response.finish_reason, FinishReason::Stop);
    assert_eq!(
        response.structured_output,
        Some(json!({"summary": "Done", "done": true}))
    );
}

#[test]
fn rejects_invalid_json_and_schema_mismatch() {
    let invalid_json = structured_response("stop", Some("{"), &status_schema());
    assert!(matches!(invalid_json, Err(ModelError::Provider { .. })));

    let schema_mismatch = structured_response(
        "stop",
        Some("{\"summary\":3,\"done\":true}"),
        &status_schema(),
    );
    assert!(matches!(schema_mismatch, Err(ModelError::Provider { .. })));
}

#[test]
fn non_stop_response_preserves_finish_without_parsing_partial_json() {
    for (finish, expected) in [
        ("length", FinishReason::Truncated),
        ("content_filter", FinishReason::Filtered),
        ("custom", FinishReason::Other("custom".to_owned())),
    ] {
        let response = structured_response(finish, Some("{"), &status_schema())
            .expect("non-stop response should not parse structured output");

        assert_eq!(response.finish_reason, expected);
        assert_eq!(response.structured_output, None);
    }
}

#[test]
fn tool_call_finish_skips_partial_structured_output() {
    let response = parse_response(
        KIMI_K3,
        KIMI_K3,
        ThinkingLevel::Max,
        json!({
            "choices": [{
                "finish_reason": "tool_calls",
                "message": {
                    "content": "{",
                    "reasoning_content": "hidden",
                    "tool_calls": [{
                        "id": "call_1",
                        "type": "function",
                        "function": {
                            "name": "memory_read",
                            "arguments": "{\"path\":\"root\"}"
                        }
                    }]
                }
            }]
        }),
        Some(&status_schema()),
    )
    .expect("valid tool-call response should not parse structured output");

    assert_eq!(response.finish_reason, FinishReason::ToolCalls);
    assert_eq!(response.tool_calls.len(), 1);
    assert_eq!(response.structured_output, None);
}

fn structured_response(
    finish_reason: &str,
    content: Option<&str>,
    schema: &StructuredOutputSchema,
) -> Result<ai_interface::ModelResponse, ModelError> {
    parse_response(
        KIMI_K3,
        KIMI_K3,
        ThinkingLevel::Max,
        json!({
            "choices": [{
                "finish_reason": finish_reason,
                "message": {
                    "content": content,
                    "reasoning_content": "hidden"
                }
            }]
        }),
        Some(schema),
    )
}

fn status_schema() -> StructuredOutputSchema {
    StructuredOutputSchema {
        name: "status_response".to_owned(),
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
