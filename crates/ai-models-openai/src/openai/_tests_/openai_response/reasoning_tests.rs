//! Reasoning output item response tests.

use ai_interface::FinishReason;
use serde_json::json;

use crate::openai::replay_state::{
    FunctionCallState, ReasoningState, ReasoningSummary, ReplayState,
};
use crate::openai::stream_support::replay_states;

use super::{Effort, response_from_fixture};

#[tokio::test]
async fn ignores_reasoning_output_items() {
    let response = response_from_fixture(
        "gpt-5.5-thinking-extra-high",
        "gpt-5.5",
        Some(Effort::ExtraHigh),
        json!({
            "status": "completed",
            "output": [
                {
                    "type": "reasoning",
                    "id": "rs_hidden",
                    "summary": [{ "type": "summary_text", "text": "hidden" }]
                },
                {
                    "type": "message",
                    "role": "assistant",
                    "content": [{ "type": "output_text", "text": "Visible" }]
                },
                {
                    "type": "function_call",
                    "call_id": "call_1",
                    "name": "memory_read",
                    "arguments": "{\"path\":\"root\"}"
                }
            ]
        }),
        None,
    )
    .await
    .expect("mixed reasoning response should parse");

    assert_eq!(response.text(), "Visible");
    assert_eq!(response.finish_reason, FinishReason::ToolCalls);
    assert_eq!(response.tool_calls().len(), 1);
    assert_eq!(
        response.catalog_model_id.as_deref(),
        Some("gpt-5.5-thinking-extra-high")
    );
}

#[tokio::test]
async fn preserves_reasoning_items_for_stateless_tool_continuation() {
    let response = response_from_fixture(
        "gpt-5.5-thinking-extra-high",
        "gpt-5.5",
        Some(Effort::ExtraHigh),
        json!({
            "status": "completed",
            "output": [
                {
                    "type": "reasoning",
                    "id": "rs_1",
                    "summary": [{ "type": "summary_text", "text": "Need the tool." }],
                    "encrypted_content": "encrypted-reasoning"
                },
                {
                    "type": "function_call",
                    "call_id": "call_1",
                    "name": "memory_read",
                    "arguments": "{\"path\":\"root\"}"
                }
            ]
        }),
        None,
    )
    .await
    .expect("reasoning response should parse");

    assert_eq!(response.finish_reason, FinishReason::ToolCalls);
    assert_eq!(response.tool_calls().len(), 1);
    assert_eq!(
        replay_states(&response),
        vec![
            ReplayState::Reasoning(ReasoningState {
                id: "rs_1".to_owned(),
                summary: vec![ReasoningSummary {
                    kind: "summary_text".to_owned(),
                    text: "Need the tool.".to_owned(),
                }],
                encrypted_content: Some("encrypted-reasoning".to_owned()),
            }),
            ReplayState::FunctionCall(FunctionCallState {
                id: None,
                call_id: "call_1".to_owned(),
                name: "memory_read".to_owned(),
                arguments: "{\"path\":\"root\"}".to_owned(),
            })
        ]
    );
}

#[tokio::test]
async fn preserves_function_call_items_for_stateless_replay_context() {
    let response = response_from_fixture(
        "gpt-5.5-thinking-extra-high",
        "gpt-5.5",
        Some(Effort::ExtraHigh),
        json!({
            "status": "completed",
            "output": [
                {
                    "type": "reasoning",
                    "id": "rs_1",
                    "summary": [{ "type": "summary_text", "text": "Need the tool." }],
                    "encrypted_content": "encrypted-reasoning"
                },
                {
                    "type": "function_call",
                    "id": "fc_1",
                    "call_id": "call_1",
                    "name": "memory_read",
                    "arguments": "{\n  \"path\": \"root\"\n}"
                }
            ]
        }),
        None,
    )
    .await
    .expect("function-call response should parse");

    assert_eq!(response.finish_reason, FinishReason::ToolCalls);
    assert_eq!(response.tool_calls().len(), 1);
    assert_eq!(
        serde_json::to_value(&replay_states(&response)[1]).expect("context should serialize")["id"],
        "fc_1"
    );
    assert_eq!(
        replay_states(&response),
        vec![
            ReplayState::Reasoning(ReasoningState {
                id: "rs_1".to_owned(),
                summary: vec![ReasoningSummary {
                    kind: "summary_text".to_owned(),
                    text: "Need the tool.".to_owned(),
                }],
                encrypted_content: Some("encrypted-reasoning".to_owned()),
            }),
            ReplayState::FunctionCall(FunctionCallState {
                id: Some("fc_1".to_owned()),
                call_id: "call_1".to_owned(),
                name: "memory_read".to_owned(),
                arguments: "{\n  \"path\": \"root\"\n}".to_owned(),
            })
        ]
    );
}

#[tokio::test]
async fn preserves_assistant_message_phase_for_stateless_replay_context() {
    let response = response_from_fixture(
        "gpt-5.5-thinking-extra-high",
        "gpt-5.5",
        Some(Effort::ExtraHigh),
        json!({
            "status": "completed",
            "output": [
                {
                    "type": "message",
                    "role": "assistant",
                    "phase": "commentary",
                    "content": [{ "type": "output_text", "text": "I'll inspect memory." }]
                },
                {
                    "type": "function_call",
                    "id": "fc_1",
                    "call_id": "call_1",
                    "name": "memory_read",
                    "arguments": "{\"path\":\"root\"}"
                }
            ]
        }),
        None,
    )
    .await
    .expect("preamble response should parse");

    let provider_context = serde_json::to_value(
        response
            .parts
            .iter()
            .filter_map(|part| part.provider_state())
            .collect::<Vec<_>>(),
    )
    .expect("context should serialize");
    assert_eq!(provider_context[0]["provider"], "openai");
    assert_eq!(provider_context[0]["payload"]["phase"], "commentary");
    assert_eq!(provider_context[1]["payload"]["call_id"], "call_1");
    assert_eq!(response.text(), "I'll inspect memory.");
}

#[tokio::test]
async fn preserves_phase_less_assistant_message_order_for_stateless_replay_context() {
    let response = response_from_fixture(
        "gpt-5.5-thinking-extra-high",
        "gpt-5.5",
        Some(Effort::ExtraHigh),
        json!({
            "status": "completed",
            "output": [
                {
                    "type": "message",
                    "role": "assistant",
                    "content": [{ "type": "output_text", "text": "I'll inspect memory." }]
                },
                {
                    "type": "function_call",
                    "id": "fc_1",
                    "call_id": "call_1",
                    "name": "memory_read",
                    "arguments": "{\"path\":\"root\"}"
                }
            ]
        }),
        None,
    )
    .await
    .expect("phase-less preamble response should parse");

    let provider_context = serde_json::to_value(
        response
            .parts
            .iter()
            .filter_map(|part| part.provider_state())
            .collect::<Vec<_>>(),
    )
    .expect("context should serialize");
    let message_context = provider_context[0]["payload"]
        .as_object()
        .expect("message context should serialize as an object");
    assert_eq!(provider_context[0]["provider"], "openai");
    assert!(!message_context.contains_key("phase"));
    assert_eq!(provider_context[1]["payload"]["call_id"], "call_1");
    assert_eq!(response.text(), "I'll inspect memory.");
}
