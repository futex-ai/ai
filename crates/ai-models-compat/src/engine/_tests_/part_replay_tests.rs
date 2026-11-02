//! Provider isolation and own-state validation at the model boundary.

use std::sync::Arc;

use ai_interface::{
    ConversationMessage, ConversationPart, ConversationRole, DynModel, ModelError, ModelRequest,
    ProviderId, ProviderState, ToolCall,
};
use ai_models_core::test_support::{fixtures_for_buffered_responses, recording_streaming_client};
use json_http::{DynJsonHttpClient, JsonHttpResponse, TransportBackedJsonHttpClient};
use serde_json::json;
use unimock::Unimock;

use crate::{
    deepseek::DeepSeekModel,
    kimi::KimiModel,
    minimax::{MINIMAX_M3, MiniMaxModel},
    qwen::QwenModel,
    xai::{GROK_4_5, XaiModel},
};

const PROVIDERS: [&str; 5] = ["deepseek", "kimi", "qwen", "minimax", "xai"];

fn model(provider: &str, http: DynJsonHttpClient) -> DynModel {
    match provider {
        "deepseek" => Arc::new(DeepSeekModel::new(http, "key")),
        "kimi" => Arc::new(KimiModel::new(http, "key")),
        "qwen" => Arc::new(QwenModel::new(http, "key")),
        "minimax" => Arc::new(MiniMaxModel::new(http, MINIMAX_M3, "key")),
        "xai" => Arc::new(XaiModel::new(http, GROK_4_5, "key")),
        _ => panic!("unknown test provider"),
    }
}

fn request(message: ConversationMessage) -> ModelRequest {
    ModelRequest {
        system_prompt: String::new(),
        messages: vec![message],
        tools: Vec::new(),
        response_schema: None,
        controls: Default::default(),
    }
}

fn state(provider: &str, version: u32) -> ProviderState {
    ProviderState {
        provider: ProviderId::from(provider.to_owned()),
        model: "another-model".to_owned(),
        version,
        payload: json!({"content": 4, "reasoning_content": 7, "tool_call_id": 5})
            .as_object()
            .expect("object")
            .clone(),
    }
}

#[tokio::test]
async fn rejects_invalid_own_state_on_portable_and_reasoning_parts_before_transport() {
    for provider in PROVIDERS {
        for (version, expected) in [
            (0, "unsupported provider state version"),
            (1, "invalid provider state payload"),
            (2, "invalid provider state payload"),
            (3, "unsupported provider state version"),
        ] {
            for part in [
                ConversationPart::text("hello"),
                ConversationPart::reasoning("private"),
                ConversationPart::from_tool_call(ToolCall {
                    id: "call_1".to_owned(),
                    name: "lookup".to_owned(),
                    input: json!({}),
                    operation_id: None,
                }),
            ] {
                let http = Arc::new(TransportBackedJsonHttpClient::new(Arc::new(Unimock::new(
                    (),
                ))));
                let model = model(provider, http);
                let input = request(ConversationMessage::new(
                    ConversationRole::Assistant,
                    vec![part.with_provider_state(state(provider, version))],
                ));
                let error = model
                    .stream(&input)
                    .await
                    .err()
                    .expect("own state fails before stream opens");
                assert!(
                    matches!(&error, ModelError::Provider { provider: actual, message, .. } if actual == provider && message == expected),
                    "{provider} version {version}: {error:?}"
                );
            }
        }
    }
}

#[tokio::test]
async fn sends_foreign_text_and_calls_portably_and_skips_unowned_reasoning() {
    for provider in PROVIDERS {
        let response = JsonHttpResponse {
            headers: Default::default(),
            status: 200,
            body: json!({"choices": [{"message": {"content": "Done"}, "finish_reason": "stop"}]}),
        };
        let (http, requests) =
            recording_streaming_client(fixtures_for_buffered_responses(vec![response]));
        let model = model(provider, http);
        let foreign = state("other-provider", 0);
        let mut message = ConversationMessage::assistant(
            "portable",
            vec![ToolCall {
                id: "call_1".to_owned(),
                name: "lookup".to_owned(),
                input: json!({"id": 7}),
                operation_id: None,
            }],
        );
        for part in &mut message.parts {
            *part.provider_state_mut() = Some(foreign.clone());
        }
        message
            .parts
            .insert(0, ConversationPart::reasoning("private without state"));
        message.parts.insert(
            1,
            ConversationPart::reasoning("private foreign").with_provider_state(foreign),
        );
        model
            .complete(&request(message))
            .await
            .expect("foreign state is opaque");
        let requests = requests.lock().expect("requests");
        assert_eq!(
            serde_json::from_slice::<serde_json::Value>(
                requests[0]
                    .body
                    .as_ref()
                    .expect("body")
                    .as_json_bytes()
                    .expect("JSON request body")
            )
            .expect("valid JSON request body")["messages"],
            json!([{
                "role": "assistant", "content": "portable", "tool_calls": [{ "id": "call_1", "type": "function", "function": { "name": "lookup", "arguments": "{\"id\":7}" } }]
            }]),
            "{provider}"
        );
    }
}

#[tokio::test]
async fn streams_part_state_and_replays_other_model_state_with_original_wire_forms() {
    let raw_arguments = "{\n  \"id\": 7\n}";
    for provider in PROVIDERS {
        let first = JsonHttpResponse {
            headers: Default::default(),
            status: 200,
            body: json!({ "choices": [{
                "finish_reason": "tool_calls", "message": {
                    "content": null, "reasoning_content": "private reasoning",
                    "reasoning_details": [{"type": "reasoning.text", "id": "reasoning_1", "format": "native", "index": 0, "text": "snapshot"}],
                    "tool_calls": [{ "id": "call_1", "function": {"name": "lookup", "arguments": raw_arguments} }]
                }
            }]}),
        };
        let second = JsonHttpResponse {
            headers: Default::default(),
            status: 200,
            body: json!({"choices": [{"message": {"content": "Done"}, "finish_reason": "stop"}]}),
        };
        let (http, requests) =
            recording_streaming_client(fixtures_for_buffered_responses(vec![first, second]));
        let model = model(provider, http);
        let response = model
            .complete(&request(ConversationMessage::user("lookup")))
            .await
            .expect("tool response");
        let call_state = response
            .parts
            .iter()
            .find_map(|part| match part {
                ConversationPart::ToolCall { provider_state, .. } => provider_state.as_ref(),
                _ => None,
            })
            .expect("tool state belongs to the call part");
        assert_eq!(
            (call_state.provider.as_str(), call_state.version),
            (provider, 2),
            "{provider}"
        );
        assert_eq!(
            call_state.payload["call"]["arguments"], raw_arguments,
            "{provider}"
        );
        let serialized =
            serde_json::to_value(response.to_assistant_message()).expect("message serializes");
        let mut message: ConversationMessage =
            serde_json::from_value(serialized).expect("stored message reads");
        for state in message
            .parts
            .iter_mut()
            .filter_map(|part| part.provider_state_mut().as_mut())
        {
            state.model = "other-model-of-this-provider".to_owned();
        }
        let mut input = request(message);
        input
            .messages
            .push(ConversationMessage::tool("result", "lookup", "call_1"));
        model
            .complete(&input)
            .await
            .expect("state from another model replays");
        let requests = requests.lock().expect("requests");
        let assistant = &serde_json::from_slice::<serde_json::Value>(
            requests[1]
                .body
                .as_ref()
                .expect("body")
                .as_json_bytes()
                .expect("JSON request body"),
        )
        .expect("valid JSON request body")["messages"][0];
        assert_eq!(
            assistant["tool_calls"][0]["function"]["arguments"],
            if matches!(provider, "deepseek" | "kimi" | "qwen") {
                raw_arguments
            } else {
                "{\"id\":7}"
            },
            "{provider}"
        );
        assert_eq!(
            assistant.get("content").cloned(),
            match provider {
                "deepseek" => Some(json!("")),
                "kimi" | "qwen" => Some(serde_json::Value::Null),
                _ => None,
            },
            "{provider}"
        );
        assert_eq!(
            assistant.get("reasoning_content").cloned(),
            if provider == "xai" {
                None
            } else {
                Some(json!("private reasoning"))
            },
            "{provider}"
        );
        assert_eq!(
            assistant.get("reasoning_details").cloned(),
            if provider == "minimax" {
                Some(
                    json!([{"type": "reasoning.text", "id": "reasoning_1", "format": "native", "index": 0, "text": "snapshot"}]),
                )
            } else {
                None
            },
            "{provider}"
        );
    }
}
