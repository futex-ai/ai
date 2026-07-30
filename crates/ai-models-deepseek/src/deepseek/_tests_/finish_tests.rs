//! DeepSeek finish-reason and resource-safety tests.

use ai_interface::{FinishReason, ModelError};
use ai_models_core::ThinkingLevel;
use serde_json::{Value, json};

use crate::DEEPSEEK_V4_PRO;

use super::response::parse_response;

#[test]
fn maps_every_normalized_finish_reason() {
    let cases = [
        (Some("stop"), FinishReason::Stop, false),
        (Some("tool_calls"), FinishReason::ToolCalls, true),
        (Some("length"), FinishReason::Truncated, false),
        (Some("content_filter"), FinishReason::Filtered, false),
        (
            Some("future_reason"),
            FinishReason::Other("future_reason".to_owned()),
            false,
        ),
        (None, FinishReason::Other("missing".to_owned()), false),
    ];

    for (raw_reason, expected, exposes_tools) in cases {
        let response =
            parse(response_body(raw_reason, exposes_tools)).expect("finish response should parse");

        assert_eq!(response.finish_reason, expected);
        assert_eq!(response.tool_calls.len(), usize::from(exposes_tools));
    }
}

#[test]
fn insufficient_system_resource_is_transient_before_tool_parsing() {
    let error = parse(json!({
        "choices": [{
            "finish_reason": "insufficient_system_resource",
            "message": {
                "content": "partial",
                "tool_calls": [{
                    "id": "call_1",
                    "type": "function",
                    "function": {"name": "memory_read"}
                }]
            }
        }]
    }))
    .expect_err("resource-limited response should be retryable");

    assert!(matches!(
        error,
        ModelError::TransientProvider {
            provider,
            model_id,
            ..
        } if provider == "deepseek" && model_id == DEEPSEEK_V4_PRO
    ));
}

fn parse(body: Value) -> Result<ai_interface::ModelResponse, ModelError> {
    parse_response(
        DEEPSEEK_V4_PRO,
        DEEPSEEK_V4_PRO,
        ThinkingLevel::Disabled,
        body,
        None,
    )
}

fn response_body(raw_reason: Option<&str>, valid_arguments: bool) -> Value {
    let arguments = if valid_arguments {
        "{\"path\":\"root\"}"
    } else {
        "{invalid"
    };
    let mut choice = json!({
        "message": {
            "content": "Done",
            "tool_calls": [{
                "id": "call_1",
                "type": "function",
                "function": {
                    "name": "memory_read",
                    "arguments": arguments
                }
            }]
        }
    });
    if let Some(raw_reason) = raw_reason {
        choice["finish_reason"] = json!(raw_reason);
    }
    json!({"choices": [choice]})
}
