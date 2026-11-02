//! Tests for OpenAI Responses reasoning replay request mapping.

use ai_interface::{ConversationMessage, Model, ModelRequest, read_stored_message};
use serde_json::json;

use crate::openai::stream_support::client_for_buffered_bodies;
use crate::openai::stream_support::custom_spec;

use super::{OpenAiLegacyConverter, OpenAiModel};

#[tokio::test]
async fn replays_openai_reasoning_context_before_tool_outputs() {
    let (http_client, requests) = client_for_buffered_bodies(vec![openai_text_response("Done")]);
    let model = OpenAiModel::new(http_client, custom_spec("gpt-5.5"), "sk-openai");
    let assistant = read_stored_message(json!({
        "role":"assistant","content":"","tool_calls":[{"id":"call_1","name":"memory_read","input":{"path":"root"}}],
        "provider_context":[
            {"type":"deep_seek_assistant_message","content":"DeepSeek-owned content","reasoning_content":"DeepSeek-owned reasoning","tool_calls":[{"id":"deepseek_call","name":"ignored","arguments":"{\"foreign\":true}"}]},
            {"type":"kimi_assistant_message","reasoning_content":"Kimi-owned context","tool_calls":[{"id":"kimi_call","name":"ignored","arguments":"{}"}]},
            {"type":"qwen_assistant_message","content":"Qwen-owned content","reasoning_content":"Qwen-owned reasoning","tool_calls":[{"id":"qwen_call","name":"ignored","arguments":"{}"}]},
            {"type":"openai_reasoning","id":"rs_1","summary":[{"type":"summary_text","text":"Need the memory tool."}],"encrypted_content":"encrypted-reasoning"},
            {"type":"openai_function_call","id":"fc_1","call_id":"call_1","name":"memory_read","arguments":"{\n  \"path\": \"root\"\n}"}
        ]
    }), &[&OpenAiLegacyConverter]).unwrap();

    model
        .complete(&ModelRequest {
            system_prompt: "system".to_owned(),
            messages: vec![
                ConversationMessage::user("start"),
                assistant,
                ConversationMessage::tool("{\"ok\":true}", "memory_read", "call_1"),
            ],
            tools: Vec::new(),
            response_schema: None,
            controls: Default::default(),
        })
        .await
        .expect("OpenAI response should parse");

    let requests = requests
        .lock()
        .expect("requests lock should not be poisoned");
    let input = &serde_json::from_slice::<serde_json::Value>(
        requests[0]
            .body
            .as_ref()
            .expect("body present")
            .as_json_bytes()
            .expect("JSON request body"),
    )
    .expect("valid JSON request body")["input"];
    assert_eq!(input[1]["type"], "reasoning");
    assert_eq!(input[1]["id"], "rs_1");
    assert_eq!(input[1]["summary"][0]["type"], "summary_text");
    assert_eq!(input[1]["encrypted_content"], "encrypted-reasoning");
    assert_eq!(input[2]["type"], "function_call");
    assert_eq!(input[2]["id"], "fc_1");
    assert_eq!(input[2]["call_id"], "call_1");
    assert_eq!(input[2]["arguments"], "{\n  \"path\": \"root\"\n}");
    assert_eq!(input[3]["type"], "function_call_output");
    assert_eq!(input[3]["call_id"], "call_1");
    assert_eq!(function_call_count(input), 1);
    assert!(!input.to_string().contains("DeepSeek-owned"));
    assert!(!input.to_string().contains("Qwen-owned"));
}

#[tokio::test]
async fn replays_assistant_message_phase_before_function_call_context() {
    let (http_client, requests) = client_for_buffered_bodies(vec![openai_text_response("Done")]);
    let model = OpenAiModel::new(http_client, custom_spec("gpt-5.5"), "sk-openai");
    let assistant = read_stored_message(json!({
        "role":"assistant","content":"I'll inspect memory.","tool_calls":[{"id":"call_1","name":"memory_read","input":{"path":"root"}}],
        "provider_context":[{"type":"openai_message","phase":"commentary"},{"type":"openai_function_call","id":"fc_1","call_id":"call_1","name":"memory_read","arguments":"{\"path\":\"root\"}"}]
    }), &[&OpenAiLegacyConverter]).unwrap();

    model
        .complete(&ModelRequest {
            system_prompt: "system".to_owned(),
            messages: vec![
                ConversationMessage::user("start"),
                assistant,
                ConversationMessage::tool("{\"ok\":true}", "memory_read", "call_1"),
            ],
            tools: Vec::new(),
            response_schema: None,
            controls: Default::default(),
        })
        .await
        .expect("OpenAI response should parse");

    let requests = requests
        .lock()
        .expect("requests lock should not be poisoned");
    let input = &serde_json::from_slice::<serde_json::Value>(
        requests[0]
            .body
            .as_ref()
            .expect("body present")
            .as_json_bytes()
            .expect("JSON request body"),
    )
    .expect("valid JSON request body")["input"];
    assert_eq!(input[1]["role"], "assistant");
    assert_eq!(input[1]["phase"], "commentary");
    assert_eq!(input[1]["content"], "I'll inspect memory.");
    assert_eq!(input[2]["type"], "function_call");
    assert_eq!(input[2]["id"], "fc_1");
    assert_eq!(input[3]["type"], "function_call_output");
    assert_eq!(function_call_count(input), 1);
}

#[tokio::test]
async fn replays_phase_less_assistant_message_before_function_call_context() {
    let (http_client, requests) = client_for_buffered_bodies(vec![openai_text_response("Done")]);
    let model = OpenAiModel::new(http_client, custom_spec("gpt-5.5"), "sk-openai");
    let assistant = read_stored_message(json!({
        "role":"assistant","content":"I'll inspect memory.","tool_calls":[{"id":"call_1","name":"memory_read","input":{"path":"root"}}],
        "provider_context":[{"type":"openai_message"},{"type":"openai_function_call","id":"fc_1","call_id":"call_1","name":"memory_read","arguments":"{\"path\":\"root\"}"}]
    }), &[&OpenAiLegacyConverter]).unwrap();

    model
        .complete(&ModelRequest {
            system_prompt: "system".to_owned(),
            messages: vec![
                ConversationMessage::user("start"),
                assistant,
                ConversationMessage::tool("{\"ok\":true}", "memory_read", "call_1"),
            ],
            tools: Vec::new(),
            response_schema: None,
            controls: Default::default(),
        })
        .await
        .expect("OpenAI response should parse");

    let requests = requests
        .lock()
        .expect("requests lock should not be poisoned");
    let input = &serde_json::from_slice::<serde_json::Value>(
        requests[0]
            .body
            .as_ref()
            .expect("body present")
            .as_json_bytes()
            .expect("JSON request body"),
    )
    .expect("valid JSON request body")["input"];
    let assistant_message = input[1]
        .as_object()
        .expect("assistant message should serialize as an object");
    assert_eq!(input[1]["role"], "assistant");
    assert_eq!(input[1]["content"], "I'll inspect memory.");
    assert!(!assistant_message.contains_key("phase"));
    assert_eq!(input[2]["type"], "function_call");
    assert_eq!(input[2]["id"], "fc_1");
    assert_eq!(input[3]["type"], "function_call_output");
    assert_eq!(function_call_count(input), 1);
}

fn function_call_count(input: &serde_json::Value) -> usize {
    input
        .as_array()
        .expect("input array")
        .iter()
        .filter(|item| item["type"] == "function_call")
        .count()
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
