//! TypeSafe judgment request mapping tests.

use std::collections::BTreeMap;

use ai_interface::{JudgmentConditionCriteria, JudgmentContent, JudgmentQuestion, JudgmentRequest};
use serde_json::{Value, json};

use super::build_request;

#[test]
fn every_question_variant_maps_to_the_exact_wire_shape() {
    let request = JudgmentRequest {
        state: JudgmentContent::try_from(json!({
            "ticket": "Payouts have failed for three days",
            "attempts": 4,
            "escalated": true
        }))
        .expect("object state should be supported"),
        questions: BTreeMap::from([
            (
                "department".to_owned(),
                JudgmentQuestion::choice(
                    "Which team should handle this?",
                    [
                        ("billing", Some(JudgmentContent::from("Payments"))),
                        ("technical", None),
                    ],
                ),
            ),
            (
                "frustration".to_owned(),
                JudgmentQuestion::score(
                    "How frustrated is the author?",
                    [
                        Some(JudgmentContent::from("Calm")),
                        None,
                        Some(JudgmentContent::from("Very angry")),
                    ],
                ),
            ),
            (
                "is_urgent".to_owned(),
                JudgmentQuestion::condition(
                    "Does the ticket convey urgency?",
                    Some(JudgmentConditionCriteria::new(
                        Some(JudgmentContent::from("Time-sensitive")),
                        Some(JudgmentContent::try_from(json!(["no urgency"])).expect("array")),
                    )),
                ),
            ),
        ]),
    };

    let body = serde_json::to_value(build_request("jev-latest", &request))
        .expect("wire request should serialize");

    assert_eq!(
        body,
        json!({
            "state": {
                "ticket": "Payouts have failed for three days",
                "attempts": 4,
                "escalated": true
            },
            "model": "jev-latest",
            "questions": {
                "department": {
                    "type": "choice",
                    "instructions": "Which team should handle this?",
                    "criteria": {"billing": "Payments", "technical": null}
                },
                "frustration": {
                    "type": "score",
                    "instructions": "How frustrated is the author?",
                    "criteria": ["Calm", null, "Very angry"]
                },
                "is_urgent": {
                    "type": "noul",
                    "instructions": "Does the ticket convey urgency?",
                    "criteria": {"true": "Time-sensitive", "false": ["no urgency"]}
                }
            }
        })
    );
}

#[test]
fn absent_and_partial_condition_criteria_omit_only_optional_fields() {
    let request = JudgmentRequest {
        state: JudgmentContent::try_from(json!(["verbatim", null, 3]))
            .expect("array state should be supported"),
        questions: BTreeMap::from([
            (
                "absent".to_owned(),
                JudgmentQuestion::condition("No rubric", None),
            ),
            (
                "partial".to_owned(),
                JudgmentQuestion::condition(
                    "Partial rubric",
                    Some(JudgmentConditionCriteria::new(
                        Some(JudgmentContent::from("yes evidence")),
                        None,
                    )),
                ),
            ),
        ]),
    };

    let body = serde_json::to_value(build_request("jev-custom", &request))
        .expect("wire request should serialize");

    assert_eq!(
        body,
        json!({
            "state": ["verbatim", null, 3],
            "model": "jev-custom",
            "questions": {
                "absent": {
                    "type": "noul",
                    "instructions": "No rubric"
                },
                "partial": {
                    "type": "noul",
                    "instructions": "Partial rubric",
                    "criteria": {"true": "yes evidence"}
                }
            }
        })
    );
}

#[test]
fn text_state_and_model_are_verbatim_without_extra_fields() {
    let request = JudgmentRequest {
        state: "  preserve surrounding whitespace  ".into(),
        questions: BTreeMap::from([(
            "condition".to_owned(),
            JudgmentQuestion::condition("Question", None),
        )]),
    };

    let body: Value = serde_json::to_value(build_request("jev-unlisted", &request))
        .expect("wire request should serialize");

    assert_eq!(body["state"], "  preserve surrounding whitespace  ");
    assert_eq!(body["model"], "jev-unlisted");
    assert_eq!(
        body.as_object()
            .expect("request should be an object")
            .keys()
            .collect::<Vec<_>>(),
        vec!["model", "questions", "state"]
    );
}
