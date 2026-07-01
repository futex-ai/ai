//! OpenAI Responses response parser edge case tests.

use ai_models_core::ThinkingLevel;
use serde_json::json;

use super::response::parse_response;

mod finish_tests;
mod reasoning_tests;
mod usage_tests;

fn openai_text_body(text: &str) -> serde_json::Value {
    json!({
        "status": "completed",
        "output": [{
            "type": "message",
            "role": "assistant",
            "content": [{ "type": "output_text", "text": text }]
        }]
    })
}

fn openai_function_call_body(arguments: &str) -> serde_json::Value {
    json!({
        "status": "completed",
        "output": [{
            "type": "function_call",
            "call_id": "call_1",
            "name": "memory_read",
            "arguments": arguments
        }]
    })
}
