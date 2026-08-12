//! DeepSeek one-turn request mapping tests.

use std::{collections::BTreeMap, sync::Arc, time::Duration};

use ai_interface::{ConversationMessage, ConversationRole, Model, ModelRequest};
use json_http::StaticHeaderAuth;

use super::{DeepSeekModel, test_support::recording_http_client};
use crate::DEEPSEEK_V4_PRO;

#[tokio::test]
async fn sends_bearer_auth_to_exact_endpoint_with_non_streaming_body() {
    let (http_client, requests) =
        recording_http_client(super::test_support::successful_response(Some("Done")));
    DeepSeekModel::new(http_client, "deepseek-secret")
        .complete(&super::test_support::simple_request())
        .await
        .expect("DeepSeek response should parse");

    let requests = requests
        .lock()
        .expect("requests lock should not be poisoned");
    let request = &requests[0];
    let body = request.body.as_ref().expect("body should be present");
    let object = body
        .as_json()
        .expect("JSON body")
        .as_object()
        .expect("object");

    assert_eq!(request.url, "https://api.deepseek.com/chat/completions");
    assert_eq!(request.timeout, Duration::from_secs(10 * 60));
    assert_eq!(
        request.headers.get("Authorization").map(String::as_str),
        Some("Bearer deepseek-secret")
    );
    assert_eq!(body["model"], DEEPSEEK_V4_PRO);
    assert_eq!(body["stream"], false);
    for omitted in [
        "temperature",
        "top_p",
        "max_tokens",
        "frequency_penalty",
        "presence_penalty",
        "stop",
        "logprobs",
        "top_logprobs",
        "tool_choice",
    ] {
        assert!(!object.contains_key(omitted), "unexpected `{omitted}`");
    }
}

#[tokio::test]
async fn applies_custom_auth_and_serializes_plain_roles_in_order() {
    let (http_client, requests) =
        recording_http_client(super::test_support::successful_response(Some("Done")));
    let auth = Arc::new(StaticHeaderAuth::new(BTreeMap::from([(
        "X-DeepSeek-Test".to_owned(),
        "injected".to_owned(),
    )])));
    let model = DeepSeekModel::with_auth(http_client, auth);
    let request = ModelRequest {
        system_prompt: "Be concise.".to_owned(),
        messages: vec![
            named_message(ConversationRole::User, "hello", "caller"),
            named_message(ConversationRole::Assistant, "checking", "agent"),
            ConversationMessage::tool("{\"ok\":true}", "memory_read", "call_1"),
        ],
        tools: Vec::new(),
        response_schema: None,
        controls: Default::default(),
    };

    model
        .complete(&request)
        .await
        .expect("DeepSeek response should parse");

    let requests = requests
        .lock()
        .expect("requests lock should not be poisoned");
    let request = &requests[0];
    let messages = request.body.as_ref().expect("body")["messages"]
        .as_array()
        .expect("messages array");

    assert_eq!(
        request.headers.get("X-DeepSeek-Test").map(String::as_str),
        Some("injected")
    );
    assert_eq!(messages[0]["role"], "system");
    assert_eq!(messages[0]["content"], "Be concise.");
    assert_eq!(messages[1]["role"], "user");
    assert_eq!(messages[1]["content"], "hello");
    assert_eq!(messages[1]["name"], "caller");
    assert_eq!(messages[2]["role"], "assistant");
    assert_eq!(messages[2]["content"], "checking");
    assert_eq!(messages[2]["name"], "agent");
    assert_eq!(messages[3]["role"], "tool");
    assert_eq!(messages[3]["content"], "{\"ok\":true}");
    assert_eq!(messages[3]["tool_call_id"], "call_1");
    assert!(messages[3].get("name").is_none());
}

#[tokio::test]
async fn preserves_empty_plain_content_as_strings() {
    let (http_client, requests) =
        recording_http_client(super::test_support::successful_response(Some("Done")));
    let request = ModelRequest {
        system_prompt: String::new(),
        messages: vec![
            ConversationMessage::user(""),
            ConversationMessage::assistant("", Vec::new()),
            ConversationMessage::tool("", "memory_read", "call_1"),
        ],
        tools: Vec::new(),
        response_schema: None,
        controls: Default::default(),
    };

    DeepSeekModel::new(http_client, "deepseek-key")
        .complete(&request)
        .await
        .expect("DeepSeek response should parse");

    let requests = requests
        .lock()
        .expect("requests lock should not be poisoned");
    let messages = requests[0].body.as_ref().expect("body")["messages"]
        .as_array()
        .expect("messages array");

    for message in messages {
        assert_eq!(message["content"], "");
    }
}

fn named_message(role: ConversationRole, content: &str, name: &str) -> ConversationMessage {
    ConversationMessage {
        role,
        content: content.to_owned(),
        content_parts: Vec::new(),
        name: Some(name.to_owned()),
        tool_call_id: None,
        tool_calls: Vec::new(),
        provider_context: Vec::new(),
    }
}
