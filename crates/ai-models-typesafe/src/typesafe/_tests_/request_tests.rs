//! TypeSafe judgment request mapping tests.

use std::collections::BTreeMap;

use ai_interface::{
    JudgmentConditionCriteria, JudgmentContent, JudgmentDescription, JudgmentJson, JudgmentLevel,
    JudgmentPart, JudgmentQuestion, JudgmentRequest,
};
use serde_json::{Map, Value, json};

use super::build_request;

#[test]
fn every_question_variant_maps_to_the_exact_wire_shape() {
    let request = JudgmentRequest {
        input: JudgmentContent::try_from(json!({
            "ticket": "Payouts have failed for three days",
            "attempts": 4,
            "escalated": true
        }))
        .expect("object input should be supported")
        .into(),
        questions: BTreeMap::from([
            (
                "department".to_owned(),
                JudgmentQuestion::choice(
                    "Which team should handle this?",
                    [
                        ("billing", Some(JudgmentDescription::from("Payments"))),
                        ("technical", None),
                    ],
                ),
            ),
            (
                "frustration".to_owned(),
                JudgmentQuestion::score(
                    "How frustrated is the author?",
                    ["Calm", "Frustrated", "Very angry"],
                ),
            ),
            (
                "is_urgent".to_owned(),
                JudgmentQuestion::condition(
                    "Does the ticket convey urgency?",
                    Some(JudgmentConditionCriteria::new(
                        Some(JudgmentDescription::from("Time-sensitive")),
                        Some(JudgmentDescription::try_from(json!(["no urgency"])).expect("array")),
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
                    "criteria": ["Calm", "Frustrated", "Very angry"]
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
        input: JudgmentContent::try_from(json!(["verbatim", null, 3]))
            .expect("array input should be supported")
            .into(),
        questions: BTreeMap::from([
            (
                "absent".to_owned(),
                JudgmentQuestion::condition("No rubric", None),
            ),
            (
                "empty".to_owned(),
                JudgmentQuestion::condition(
                    "Empty rubric",
                    Some(JudgmentConditionCriteria::default()),
                ),
            ),
            (
                "no_only".to_owned(),
                JudgmentQuestion::condition(
                    "Negative rubric",
                    Some(JudgmentConditionCriteria::new(
                        None,
                        Some(JudgmentDescription::from("no evidence")),
                    )),
                ),
            ),
            (
                "partial".to_owned(),
                JudgmentQuestion::condition(
                    "Partial rubric",
                    Some(JudgmentConditionCriteria::new(
                        Some(JudgmentDescription::from("yes evidence")),
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
                "empty": {
                    "type": "noul",
                    "instructions": "Empty rubric",
                    "criteria": {}
                },
                "no_only": {
                    "type": "noul",
                    "instructions": "Negative rubric",
                    "criteria": {"false": "no evidence"}
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
fn structured_instructions_and_descriptions_map_verbatim() {
    let choice_instructions = JudgmentContent::try_from(json!({
        "task": "route",
        "signals": ["invoice", "charge"]
    }))
    .expect("object instructions should be supported");
    let score_instructions = JudgmentContent::try_from(json!([
        "Assess urgency",
        {"window_hours": 24}
    ]))
    .expect("array instructions should be supported");
    let option_description = JudgmentDescription::try_from(json!({
        "owner": "payments",
        "priority": 1
    }))
    .expect("object option description should be supported");
    let level_description = JudgmentDescription::try_from(json!({
        "requires_page": true
    }))
    .expect("object level description should be supported");
    let request = JudgmentRequest {
        input: "ticket".into(),
        questions: BTreeMap::from([
            (
                "choice".to_owned(),
                JudgmentQuestion::choice(
                    choice_instructions,
                    [("billing", Some(option_description))],
                ),
            ),
            (
                "score".to_owned(),
                JudgmentQuestion::score(
                    score_instructions,
                    [
                        JudgmentLevel::from("Routine"),
                        JudgmentLevel::new("Urgent", Some("Needs action today".into())),
                        JudgmentLevel::new("critical", Some(level_description)),
                    ],
                ),
            ),
        ]),
    };

    let body = serde_json::to_value(build_request("jev-latest", &request))
        .expect("wire request should serialize");

    assert_eq!(
        body["questions"],
        json!({
            "choice": {
                "type": "choice",
                "instructions": {
                    "task": "route",
                    "signals": ["invoice", "charge"]
                },
                "criteria": {
                    "billing": {"owner": "payments", "priority": 1}
                }
            },
            "score": {
                "type": "score",
                "instructions": ["Assess urgency", {"window_hours": 24}],
                "criteria": [
                    "Routine",
                    {"label": "Urgent", "description": "Needs action today"},
                    {"label": "critical", "description": {"requires_page": true}}
                ]
            }
        })
    );
}

#[test]
fn text_state_and_model_are_verbatim_without_extra_fields() {
    let request = JudgmentRequest {
        input: "  preserve surrounding whitespace  ".into(),
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

#[test]
fn multiple_parts_use_an_untagged_array_for_state_and_instructions() {
    let content = JudgmentContent {
        parts: vec![
            JudgmentPart::Text {
                text: "first".to_owned(),
            },
            JudgmentPart::Json {
                value: JudgmentJson::Object(Map::from_iter([("second".to_owned(), json!(true))])),
            },
            JudgmentPart::Json {
                value: JudgmentJson::Array(vec![json!("third")]),
            },
        ],
    };
    let request = JudgmentRequest {
        input: content.clone().into(),
        questions: BTreeMap::from([(
            "condition".to_owned(),
            JudgmentQuestion::condition(content, None),
        )]),
    };

    let body = serde_json::to_value(build_request("jev-latest", &request)).unwrap();

    assert_eq!(
        body,
        json!({
            "state": ["first", {"second": true}, ["third"]],
            "model": "jev-latest",
            "questions": {
                "condition": {
                    "type": "noul",
                    "instructions": ["first", {"second": true}, ["third"]]
                }
            }
        })
    );
}
