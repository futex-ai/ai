//! MiniMax finish-reason safety tests.

use ai_interface::FinishReason;
use ai_models_core::ThinkingLevel;
use serde_json::{Value, json};

use crate::MINIMAX_M3;

use super::response::parse_response;

#[test]
fn maps_finish_reasons_and_suppresses_non_tool_payloads() {
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
        let result = parse_response(
            MINIMAX_M3,
            MINIMAX_M3,
            ThinkingLevel::Medium,
            response(raw_reason, exposes_tools),
            None,
        )
        .expect("finish response should parse without touching suppressed arguments");

        assert_eq!(result.finish_reason, expected);
        assert_eq!(result.tool_calls.len(), usize::from(exposes_tools));
    }
}

fn response(raw_reason: Option<&str>, valid_arguments: bool) -> Value {
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
