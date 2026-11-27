//! Decisions response rejection through the judgment model boundary.

use ai_interface::{
    ErrorKind, JudgmentAnswerProblem, JudgmentConditionCriteria, JudgmentContent, JudgmentError,
    JudgmentModel, JudgmentPart, JudgmentQuestion, JudgmentRequest, ModelSpec,
};
use serde_json::{Value, json};

use crate::{CUSTOM_DEFAULTS, OpenAiJudgmentModel, PROVIDER_ID};

use super::support::{API_KEY, client, config, model, predicate, request, response};

#[tokio::test]
async fn rejects_repeated_answer_names_and_distribution_values() {
    let cases = [
        (
            predicate(),
            json!([
                {"name":"q0","type":"predicate","probability":0.9},
                {"name":"other","type":"predicate","probability":0.1},
                {"name":"other","type":"refusal"}
            ]),
            "duplicate answer name",
        ),
        (
            predicate(),
            json!([
                {"name":"q0","type":"predicate","probability":0.9},
                {"name":"q0","type":"predicate","probability":0.9}
            ]),
            "duplicate answer name",
        ),
        (
            string_choice(),
            json!([
                {"name":"q0","type":"choice","choice":"first","confidence":1.0,"probabilities":[
                    {"value":"first","probability":0.2},
                    {"value":"first","probability":0.75},
                    {"value":"second","probability":0.25}
                ]}
            ]),
            "duplicate distribution value",
        ),
        (
            boolean_choice(),
            json!([
                {"name":"q0","type":"choice","choice":true,"confidence":1.0,"probabilities":[
                    {"value":true,"probability":0.2},
                    {"value":true,"probability":0.75},
                    {"value":false,"probability":0.25}
                ]}
            ]),
            "duplicate distribution value",
        ),
        (
            request(JudgmentQuestion::score("Rate it", ["low", "high"])),
            json!([
                {"name":"q0","type":"score","score":0.25,"confidence":1.0,"probabilities":[
                    {"value":0,"probability":0.2},
                    {"value":0,"probability":0.75},
                    {"value":1,"probability":0.25}
                ]}
            ]),
            "duplicate distribution value",
        ),
    ];
    for (request, answers, diagnostic) in cases {
        assert_invalid(&request, answers, diagnostic).await;
    }
}

#[tokio::test]
async fn rejects_choice_values_with_the_wrong_type() {
    for (request, selected, value, diagnostic) in [
        (
            string_choice(),
            json!(true),
            json!("first"),
            "expected a string choice value",
        ),
        (
            string_choice(),
            json!("first"),
            json!(true),
            "expected a string choice value",
        ),
        (
            boolean_choice(),
            json!("true"),
            json!(true),
            "expected a boolean choice value",
        ),
        (
            boolean_choice(),
            json!(true),
            json!("true"),
            "expected a boolean choice value",
        ),
    ] {
        assert_invalid(
            &request,
            json!([
                {"name":"q0","type":"choice","choice":selected,"confidence":1.0,"probabilities":[
                    {"value":value,"probability":1.0}
                ]}
            ]),
            diagnostic,
        )
        .await;
    }
    assert_invalid(
        &boolean_choice(),
        json!([
            {"name":"q0","type":"predicate","probability":0.9}
        ]),
        "expected a choice answer for a condition with criteria",
    )
    .await;
}

#[tokio::test]
async fn boolean_condition_checks_the_full_distribution_before_conversion() {
    for (selected, entries, confidence, expected) in [
        (
            true,
            json!([{"value":true,"probability":1.0}]),
            1.0,
            JudgmentAnswerProblem::MissingOption {
                label: "false".to_owned(),
            },
        ),
        (
            true,
            boolean_probabilities(0.75, 0.5),
            1.0,
            JudgmentAnswerProblem::DistributionSum { sum: 1.25 },
        ),
        (
            true,
            boolean_probabilities(0.25, 0.75),
            1.0,
            JudgmentAnswerProblem::SelectedNotMaximal {
                selected: "true".to_owned(),
                maximal: "false".to_owned(),
            },
        ),
        (
            true,
            boolean_probabilities(0.75, 0.25),
            1.5,
            JudgmentAnswerProblem::ConfidenceOutOfRange { value: 1.5 },
        ),
    ] {
        let (model, _) = model(json!({"model":"decisions-release","answers":[
            {"name":"q0","type":"choice","choice":selected,"confidence":confidence,"probabilities":entries}
        ]}));

        let result = model.judge(&boolean_choice()).await;

        assert!(
            matches!(&result, Err(JudgmentError::InvalidAnswer { core, id, problem })
                if core.kind == ErrorKind::InvalidOutput && id == "caller-id" && problem == &expected),
            "{expected:?}: {result:?}"
        );
    }
}

#[tokio::test]
async fn malformed_and_missing_requested_answers_fail() {
    assert_invalid(
        &predicate(),
        json!([
            {"name":"q0","type":"predicate","probability":"invalid"}
        ]),
        "malformed provider payload",
    )
    .await;
    assert_invalid(
        &predicate(),
        json!([
            {"name":"q0","type":"predicate","probability":0.9},
            ["other"]
        ]),
        "answer must be a JSON object",
    )
    .await;

    let (adapter, _) = model(json!({"model":"decisions-release","answers":[
        {"name":"other","type":"predicate","probability":0.9}
    ]}));
    let result = adapter.judge(&predicate()).await;

    assert!(
        matches!(&result, Err(JudgmentError::InvalidAnswer { core, id, problem: JudgmentAnswerProblem::Missing })
            if core.kind == ErrorKind::InvalidOutput && id == "caller-id"),
        "{result:?}"
    );
}

#[tokio::test]
async fn invalid_selection_redacts_option_labels_and_model_id() {
    const IMAGE: &str = "RklYVFVSRV9PUEVOSw==";
    let mut request = request(JudgmentQuestion::choice(
        "Pick one",
        [(API_KEY, None), (IMAGE, None)],
    ));
    request.input = JudgmentContent {
        parts: vec![JudgmentPart::Image {
            mime_type: "image/png".to_owned(),
            data_base64: IMAGE.to_owned(),
        }],
    }
    .into();
    let (http, _) = client(Ok(response(
        200,
        json!({"model":"decisions-release","answers":[
            {"name":"q0","type":"choice","choice":API_KEY,"confidence":1.0,"probabilities":[
                {"value":API_KEY,"probability":0.25}, {"value":IMAGE,"probability":0.75}
            ]}
        ]}),
    )));
    let spec = ModelSpec::custom(
        PROVIDER_ID,
        format!("decisions-{API_KEY}"),
        &CUSTOM_DEFAULTS,
    );
    let model = OpenAiJudgmentModel::from_config(http, spec, config());

    let result = model.judge(&request).await;

    assert!(
        matches!(&result, Err(JudgmentError::InvalidAnswer {
        core, problem: JudgmentAnswerProblem::SelectedNotMaximal { selected, maximal }, ..
    }) if core.kind == ErrorKind::InvalidOutput && core.model_id == "decisions-[redacted]"
        && selected == "[redacted]" && maximal == "[redacted]"),
        "selection diagnostics must redact credentials and image data"
    );
    let display = result.unwrap_err().to_string();
    assert!(!display.contains(API_KEY));
    assert!(!display.contains(IMAGE));
}

fn string_choice() -> JudgmentRequest {
    request(JudgmentQuestion::choice(
        "Pick one",
        [("first", None), ("second", None)],
    ))
}

fn boolean_choice() -> JudgmentRequest {
    request(JudgmentQuestion::condition(
        "Urgent?",
        Some(JudgmentConditionCriteria::default()),
    ))
}

fn boolean_probabilities(yes: f64, no: f64) -> Value {
    json!([{"value":true,"probability":yes},{"value":false,"probability":no}])
}

async fn assert_invalid(request: &JudgmentRequest, answers: Value, diagnostic: &str) {
    let (model, _) = model(json!({"model":"decisions-release","answers":answers}));
    let result = model.judge(request).await;
    assert!(
        matches!(&result, Err(error) if error.core().kind == ErrorKind::InvalidOutput
            && error.core().message.contains(diagnostic)),
        "{diagnostic}: {result:?}"
    );
}
