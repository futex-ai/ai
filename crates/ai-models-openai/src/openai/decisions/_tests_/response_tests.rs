//! Decisions answer association and token normalization tests.

use std::collections::BTreeMap;

use ai_interface::{
    JudgmentAnswer, JudgmentConditionCriteria, JudgmentModel, JudgmentQuestion, JudgmentRequest,
    ModelUsage,
};
use serde_json::json;

use super::support::{MODEL_ID, model, predicate, predicate_response};

#[tokio::test]
async fn normalizes_answers_by_name_and_splits_reported_usage() {
    let request = JudgmentRequest {
        input: "A ticket with a late payout.".into(),
        questions: BTreeMap::from([
            ("a".to_owned(), JudgmentQuestion::condition("Urgent?", None)),
            (
                "b".to_owned(),
                JudgmentQuestion::condition(
                    "Needs action?",
                    Some(JudgmentConditionCriteria::new(
                        Some("Action today".into()),
                        None,
                    )),
                ),
            ),
            (
                "c".to_owned(),
                JudgmentQuestion::choice("Team?", [("technical", None), ("billing", None)]),
            ),
            (
                "d".to_owned(),
                JudgmentQuestion::score("Severity?", ["routine", "urgent"]),
            ),
            (
                "e".to_owned(),
                JudgmentQuestion::condition("Resolved?", None),
            ),
        ]),
    };
    let body = json!({
        "model":"decisions-release",
        "answers":[
            {"name":"q4","type":"refusal"},
            {"name":"other","type":"unexpected"},
            {"name":"q3","type":"score","score":0.75,"confidence":0.8,"probabilities":[
                {"value":1,"label":"ignored","probability":0.75},
                {"value":0,"label":"also ignored","probability":0.25}
            ]},
            {"name":null,"type":"unexpected"},
            {"name":"q2","type":"choice","choice":"billing","confidence":0.9,"probabilities":[
                {"value":"billing","probability":0.875},
                {"value":"technical","probability":0.125}
            ]},
            {"name":"q1","type":"choice","choice":false,"confidence":0.8,"probabilities":[
                {"value":true,"probability":0.25},
                {"value":false,"probability":0.75}
            ]},
            {"type":"unexpected"},
            {"name":"q0","type":"predicate","probability":0.9}
        ],
        "usage":{
            "input_tokens":1500,
            "input_tokens_details":{"cached_tokens":1024,"cache_write_tokens":50},
            "output_tokens":20,
            "output_tokens_details":{"reasoning_tokens":12},
            "total_tokens":1530
        }
    });
    let (model, _) = model(body);

    let judged = model.judge(&request).await.unwrap();

    assert_eq!(judged.provider, "openai");
    assert_eq!(judged.model_id, MODEL_ID);
    assert_eq!(judged.resolved_model_id, "decisions-release");
    assert_eq!(
        judged.answers,
        BTreeMap::from([
            (
                "a".to_owned(),
                JudgmentAnswer::Condition { probability: 0.9 }
            ),
            (
                "b".to_owned(),
                JudgmentAnswer::Condition { probability: 0.25 }
            ),
            (
                "c".to_owned(),
                JudgmentAnswer::Choice {
                    selected: "billing".to_owned(),
                    probabilities: BTreeMap::from([
                        ("billing".to_owned(), 0.875),
                        ("technical".to_owned(), 0.125)
                    ]),
                    confidence: 0.9,
                }
            ),
            (
                "d".to_owned(),
                JudgmentAnswer::Score {
                    expected: 0.75,
                    probabilities: BTreeMap::from([(0, 0.25), (1, 0.75)]),
                    confidence: 0.8,
                }
            ),
            ("e".to_owned(), JudgmentAnswer::Refused),
        ])
    );
    assert_eq!(
        judged.usage,
        ModelUsage {
            input_tokens: 476,
            output_tokens: 8,
            cached_input_tokens: 1024,
            reasoning_tokens: 12,
            total_tokens: 1530,
            ..ModelUsage::default()
        }
    );
}

#[tokio::test]
async fn usage_defaults_and_subtractions_stop_at_zero() {
    let (adapter, _) = model(predicate_response());
    assert_eq!(
        adapter.judge(&predicate()).await.unwrap().usage,
        ModelUsage::default()
    );

    let mut body = predicate_response();
    body["usage"] = json!({
        "input_tokens":2, "input_tokens_details":{"cached_tokens":4},
        "output_tokens_details":{"reasoning_tokens":5}
    });
    let (adapter, _) = model(body);

    assert_eq!(
        adapter.judge(&predicate()).await.unwrap().usage,
        ModelUsage {
            cached_input_tokens: 4,
            reasoning_tokens: 5,
            total_tokens: 9,
            ..ModelUsage::default()
        }
    );
}
