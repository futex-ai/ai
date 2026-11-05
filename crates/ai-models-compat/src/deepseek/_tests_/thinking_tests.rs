//! DeepSeek thinking-control request tests.

use ai_interface::Effort;
use serde_json::{Value, json};

use crate::deepseek::DEEPSEEK_V4_PRO;

use super::{request::build_request, test_support::simple_request};

#[test]
fn serializes_disabled_thinking_without_reasoning_effort() {
    let body = request_json(Effort::Disabled);
    let object = body.as_object().expect("request object");

    assert_eq!(body["thinking"], json!({"type": "disabled"}));
    assert!(!object.contains_key("reasoning_effort"));
}

#[test]
fn serializes_enabled_high_and_max_thinking() {
    for (level, effort) in [(Effort::High, "high"), (Effort::Max, "max")] {
        let body = request_json(level);

        assert_eq!(body["thinking"], json!({"type": "enabled"}));
        assert_eq!(body["reasoning_effort"], effort);
    }
}

#[test]
fn omits_sampling_and_tool_choice_fields_for_every_thinking_mode() {
    for level in [Effort::Disabled, Effort::High, Effort::Max] {
        let body = request_json(level);
        let object = body.as_object().expect("request object");

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
}

fn request_json(thinking_level: Effort) -> Value {
    serde_json::to_value(
        build_request(
            DEEPSEEK_V4_PRO.provider_model_id.as_ref(),
            thinking_level,
            &simple_request(),
        )
        .expect("plain request should build"),
    )
    .expect("DeepSeek request should serialize")
}
