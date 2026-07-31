//! Qwen usage and structured-output response tests.

use ai_interface::{ModelError, ModelUsage, StructuredOutputSchema};
use ai_models_core::ThinkingLevel;
use serde_json::{Value, json};

use crate::QWEN_3_7_PLUS;

use super::response::parse_response;

#[test]
fn normalizes_cached_usage_and_falls_back_when_total_is_absent() {
    let with_total = parse(
        response_with_usage(json!({
            "prompt_tokens": 120,
            "completion_tokens": 30,
            "total_tokens": 150,
            "prompt_tokens_details": {"cached_tokens": 20}
        })),
        None,
    )
    .expect("usage should parse");
    assert_eq!(
        with_total.usage,
        ModelUsage {
            input_tokens: 100,
            output_tokens: 30,
            cached_input_tokens: 20,
            reasoning_tokens: 0,
            total_tokens: 150,
            estimated_cost_microusd: 0,
            cost_lines: Vec::new(),
        }
    );

    let fallback = parse(
        response_with_usage(json!({
            "prompt_tokens": 5,
            "completion_tokens": 7,
            "prompt_tokens_details": {"cached_tokens": 9}
        })),
        None,
    )
    .expect("fallback usage should parse");
    assert_eq!(fallback.usage.input_tokens, 0);
    assert_eq!(fallback.usage.cached_input_tokens, 9);
    assert_eq!(fallback.usage.output_tokens, 7);
    assert_eq!(fallback.usage.total_tokens, 16);
}

#[test]
fn validates_structured_output_locally_and_only_on_stop() {
    let schema = status_schema();
    let valid = parse(
        json!({
            "choices": [{
                "finish_reason": "stop",
                "message": {"content": "{\"done\":true}"}
            }]
        }),
        Some(&schema),
    )
    .expect("valid JSON object should parse");
    assert_eq!(valid.structured_output, Some(json!({"done": true})));

    for content in ["", "not json", "{\"done\":7}"] {
        let result = parse(
            json!({
                "choices": [{"finish_reason": "stop", "message": {"content": content}}]
            }),
            Some(&schema),
        );
        assert!(matches!(result, Err(ModelError::Provider { .. })));
    }

    let truncated = parse(
        json!({
            "choices": [{"finish_reason": "length", "message": {"content": "not json"}}]
        }),
        Some(&schema),
    )
    .expect("non-stop response should skip schema parsing");
    assert_eq!(truncated.structured_output, None);
}

#[test]
fn malformed_typed_payloads_are_internal_but_missing_choices_is_provider_failure() {
    for body in [
        json!({"choices": "invalid"}),
        json!({"choices": [{"message": {"content": 7}}]}),
        json!({"choices": [{"finish_reason": 7, "message": {"content": "Done"}}]}),
    ] {
        assert!(matches!(
            parse(body, None),
            Err(ModelError::Internal { .. })
        ));
    }
    assert!(matches!(
        parse(json!({"choices": []}), None),
        Err(ModelError::Provider { .. })
    ));
}

fn parse(
    body: Value,
    schema: Option<&StructuredOutputSchema>,
) -> Result<ai_interface::ModelResponse, ModelError> {
    parse_response(
        "catalog-plus",
        QWEN_3_7_PLUS,
        ThinkingLevel::Disabled,
        body,
        schema,
    )
}

fn response_with_usage(usage: Value) -> Value {
    json!({
        "choices": [{"finish_reason": "stop", "message": {"content": "Done"}}],
        "usage": usage
    })
}

fn status_schema() -> StructuredOutputSchema {
    StructuredOutputSchema {
        name: "status".to_owned(),
        schema: json!({
            "type": "object",
            "properties": {"done": {"type": "boolean"}},
            "required": ["done"]
        }),
    }
}
