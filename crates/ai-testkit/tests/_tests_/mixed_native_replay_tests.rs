//! Mixed version one items preserve complete native requests and OpenAI order.

use std::sync::Arc;

use ai_interface::{ConversationMessage, LegacyMessageConverter};
use ai_testkit::provider::Provider;
use serde_json::{Value, json};

use super::{converter, legacy_request_support::request_body};

#[derive(Clone, Copy)]
enum NativeCase {
    DuplicateText,
    ReorderedBlocks,
    MissingPortableCall,
    RepeatedNativeCall,
}

#[tokio::test]
async fn mixed_messages_replay_native_text_without_duplicates() {
    assert_native_replay(NativeCase::DuplicateText).await;
}

#[tokio::test]
async fn mixed_messages_keep_native_block_order() {
    assert_native_replay(NativeCase::ReorderedBlocks).await;
}

#[tokio::test]
async fn mixed_messages_keep_native_calls_without_portable_calls() {
    assert_native_replay(NativeCase::MissingPortableCall).await;
}

#[tokio::test]
async fn mixed_messages_keep_repeated_native_call_blocks() {
    assert_native_replay(NativeCase::RepeatedNativeCall).await;
}

#[tokio::test]
async fn mixed_messages_keep_openai_order_when_full_content_is_first() {
    let mut actual = Vec::new();
    let mut expected = Vec::new();
    for provider in [Provider::Anthropic, Provider::Google] {
        for all_converters in [false, true] {
            let (item, native_request) = native_item(provider, NativeCase::RepeatedNativeCall);
            let message = mixed_converter(provider, all_converters)
                .read(json!({
                    "role": "assistant", "content": "visible", "tool_calls": portable_calls(),
                    "provider_context": [
                        item,
                        {"type": "openai_reasoning", "id": "reasoning", "encrypted_content": "encrypted"},
                        {"type": "openai_message", "phase": "commentary"},
                        {"type": "openai_function_call", "id": "function", "call_id": "call", "name": "lookup", "arguments": "{\"id\":7}"}
                    ]
                }))
                .unwrap();
            let native_body = request_body(provider, message.clone()).await;
            let openai_body = request_body(Provider::Openai, message).await;
            actual.push(json!({
                "provider": provider.name(), "all_converters": all_converters,
                "native": native_body[request_field(provider)], "openai": openai_body["input"]
            }));
            expected.push(json!({
                "provider": provider.name(), "all_converters": all_converters,
                "native": native_request,
                "openai": [
                    {"type": "reasoning", "id": "reasoning", "summary": [], "encrypted_content": "encrypted"},
                    {"role": "assistant", "content": "visible", "phase": "commentary"},
                    {"type": "function_call", "id": "function", "call_id": "call", "name": "lookup", "arguments": "{\"id\":7}"}
                ]
            }));
        }
    }
    assert_eq!(actual, expected);

    let message = json!({"type": "openai_message", "phase": "commentary"});
    let call = json!({"type": "openai_function_call", "id": "function", "call_id": "call", "name": "lookup", "arguments": "{\"id\":7}"});
    let reasoning =
        json!({"type": "openai_reasoning", "id": "r1", "encrypted_content": "encrypted"});
    let later = json!({"type": "openai_reasoning", "id": "r2"});
    let native_message = json!({"role": "assistant", "content": "visible", "phase": "commentary"});
    let native_call = json!({"type": "function_call", "id": "function", "call_id": "call", "name": "lookup", "arguments": "{\"id\":7}"});
    let native_reasoning =
        json!({"type": "reasoning", "id": "r1", "summary": [], "encrypted_content": "encrypted"});
    let native_later = json!({"type": "reasoning", "id": "r2", "summary": []});
    for (items, expected) in [
        (
            vec![message.clone(), call.clone(), reasoning.clone()],
            json!([native_message, native_call, native_reasoning]),
        ),
        (
            vec![reasoning, message, later, call],
            json!([native_reasoning, native_message, native_later, native_call]),
        ),
    ] {
        for provider in [Provider::Anthropic, Provider::Google] {
            for native_installed in [false, true] {
                let (item, _) = native_item(provider, NativeCase::RepeatedNativeCall);
                let mut context = vec![item];
                context.extend(items.clone());
                let calls = json!([
                    {"id": "call", "name": "lookup", "input": {"id": 7}},
                    {"id": "unmatched", "name": "omitted", "input": {}}
                ]);
                let converter = if native_installed {
                    converter()
                } else {
                    LegacyMessageConverter::new(vec![Arc::new(
                        ai_models_openai::OpenAiLegacyConverter,
                    )])
                };
                let message = converter
                    .read(json!({
                        "role": "assistant", "content": "visible", "tool_calls": calls,
                        "provider_context": context
                    }))
                    .unwrap();
                let body = request_body(Provider::Openai, message).await;
                assert_eq!(
                    body["input"],
                    expected,
                    "{} native converter installed: {native_installed}",
                    provider.name()
                );
            }
        }
    }
}

async fn assert_native_replay(case: NativeCase) {
    let mut actual = Vec::new();
    let mut expected = Vec::new();
    for provider in [Provider::Anthropic, Provider::Google] {
        for all_converters in [false, true] {
            let (item, native_request) = native_item(provider, case);
            let content = if matches!(case, NativeCase::ReorderedBlocks) {
                "b"
            } else {
                "visible"
            };
            let calls = if matches!(
                case,
                NativeCase::DuplicateText | NativeCase::MissingPortableCall
            ) {
                json!([])
            } else {
                portable_calls()
            };
            let first = if matches!(case, NativeCase::ReorderedBlocks) {
                json!({"type": "openai_reasoning", "id": "reasoning"})
            } else {
                json!({"type": "openai_message", "phase": "commentary"})
            };
            let message = mixed_converter(provider, all_converters)
                .read(json!({
                    "role": "assistant", "content": content, "tool_calls": calls,
                    "provider_context": [first, item]
                }))
                .unwrap();
            let message: ConversationMessage =
                serde_json::from_value(serde_json::to_value(message).unwrap()).unwrap();
            let portable = json!({"text": message.text(), "calls": message.tool_calls()});
            let body = request_body(provider, message).await;
            actual.push(json!({
                "provider": provider.name(), "all_converters": all_converters,
                "portable": portable, "request": body[request_field(provider)]
            }));
            expected.push(json!({
                "provider": provider.name(), "all_converters": all_converters,
                "portable": {"text": content, "calls": calls}, "request": native_request
            }));
        }
    }
    assert_eq!(actual, expected);
}

fn mixed_converter(provider: Provider, all_converters: bool) -> LegacyMessageConverter {
    if all_converters {
        return converter();
    }
    match provider {
        Provider::Anthropic => LegacyMessageConverter::new(vec![Arc::new(
            ai_models_anthropic::AnthropicLegacyConverter,
        )]),
        Provider::Google => LegacyMessageConverter::new(vec![Arc::new(
            ai_models_google::GoogleLegacyProviderConverter,
        )]),
        _ => unreachable!(),
    }
}

fn native_item(provider: Provider, case: NativeCase) -> (Value, Value) {
    let (thought, text, call, tag, field, role) = match provider {
        Provider::Anthropic => (
            json!({"type": "thinking", "thinking": "thought", "signature": "signed"}),
            json!({"type": "text", "text": "visible"}),
            json!({"type": "tool_use", "id": "call", "name": "lookup", "input": {"id": 7}}),
            "anthropic_assistant_message",
            "blocks",
            "assistant",
        ),
        Provider::Google => (
            json!({"text": "thought", "thought": true, "thoughtSignature": "signed"}),
            json!({"text": "visible"}),
            json!({"functionCall": {"id": "call", "name": "lookup", "args": {"id": 7}}}),
            "google_assistant_message",
            "parts",
            "model",
        ),
        _ => unreachable!(),
    };
    let content = match case {
        NativeCase::DuplicateText => json!([thought, text]),
        NativeCase::ReorderedBlocks => {
            let mut before = text.clone();
            before["text"] = json!("a");
            let mut after = text;
            after["text"] = json!("b");
            json!([thought, before, call, after])
        }
        NativeCase::MissingPortableCall => json!([thought, call, text]),
        NativeCase::RepeatedNativeCall => json!([thought, text, call, call]),
    };
    let request_content = if provider == Provider::Anthropic {
        "content"
    } else {
        "parts"
    };
    (
        json!({"type": tag, field: content}),
        json!([{"role": role, request_content: content}]),
    )
}

fn request_field(provider: Provider) -> &'static str {
    if provider == Provider::Anthropic {
        "messages"
    } else {
        "contents"
    }
}

fn portable_calls() -> Value {
    json!([{"id": "call", "name": "lookup", "input": {"id": 7}}])
}
