//! Judgment request and response serde tests.

use std::collections::BTreeMap;

use serde_json::{Map, json};

use crate::{
    JudgmentAnswer, JudgmentContent, JudgmentQuestion, JudgmentRequest, JudgmentResponse,
    ModelUsage,
};

#[test]
fn request_uses_state_and_question_map_wire_shape() {
    let request = JudgmentRequest {
        state: JudgmentContent::Object(Map::from_iter([(
            "ticket".to_owned(),
            json!("Payouts have failed"),
        )])),
        questions: BTreeMap::from([(
            "urgent".to_owned(),
            JudgmentQuestion::condition("Is the ticket urgent?", None),
        )]),
    };
    let expected = json!({
        "state": {"ticket": "Payouts have failed"},
        "questions": {
            "urgent": {
                "type": "condition",
                "instructions": "Is the ticket urgent?"
            }
        }
    });

    assert_eq!(serde_json::to_value(&request).unwrap(), expected);
    assert_eq!(
        serde_json::from_value::<JudgmentRequest>(expected).unwrap(),
        request
    );
}

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
