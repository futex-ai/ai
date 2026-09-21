//! Judgment response serde tests.

use std::collections::BTreeMap;

use serde_json::json;

use crate::{JudgmentAnswer, JudgmentResponse, ModelUsage};

#[test]
fn response_uses_provider_model_answer_and_usage_wire_shape() {
    let response = JudgmentResponse {
        provider: "typesafe".to_owned(),
        model_id: "jev-latest".to_owned(),
        resolved_model_id: "jev-1.13.0".to_owned(),
        answers: BTreeMap::from([(
            "urgent".to_owned(),
            JudgmentAnswer::Condition { probability: 0.9 },
        )]),
        usage: ModelUsage {
            input_tokens: 12,
            output_tokens: 3,
            total_tokens: 15,
            ..ModelUsage::default()
        },
    };
    let expected = json!({
        "provider": "typesafe",
        "model_id": "jev-latest",
        "resolved_model_id": "jev-1.13.0",
        "answers": {
            "urgent": {"type": "condition", "probability": 0.9}
        },
        "usage": {
            "input_tokens": 12,
            "output_tokens": 3,
            "cached_input_tokens": 0,
            "reasoning_tokens": 0,
            "total_tokens": 15,
            "estimated_cost_microusd": 0
        }
    });

    assert_eq!(serde_json::to_value(&response).unwrap(), expected);
    assert_eq!(
        serde_json::from_value::<JudgmentResponse>(expected).unwrap(),
        response
    );
}
