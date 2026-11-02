//! Normalized live-judgment response validation tests.

use std::borrow::Cow;

use std::collections::{BTreeMap, BTreeSet};

use ai_interface::{
    JudgmentAnswer, JudgmentContent, JudgmentModel, JudgmentQuestion, JudgmentRequest,
    JudgmentResponse, MockJudgmentModel, ModelFeature, ModelSpec,
};

const JUDGMENT_FEATURES: &[ModelFeature] = &[ModelFeature::Judgment];

pub(super) fn validation_failures(
    spec: &ModelSpec,
    request: &JudgmentRequest,
    response: &JudgmentResponse,
) -> Vec<String> {
    let mut failures = Vec::new();

    if response.provider != "typesafe" {
        failures.push(format!(
            "{}: provider was `{}`, expected `typesafe`",
            spec.text_id(),
            response.provider
        ));
    }
    if response.model_id != spec.provider_model_id {
        failures.push(format!(
            "{}: provider model was `{}`, expected `{}`",
            spec.text_id(),
            response.model_id,
            spec.provider_model_id
        ));
    }
    if response.resolved_model_id.trim().is_empty() {
        failures.push(format!(
            "{}: resolved provider model was empty",
            spec.text_id()
        ));
    }

    let expected_ids = request.questions.keys().cloned().collect::<BTreeSet<_>>();
    let actual_ids = response.answers.keys().cloned().collect::<BTreeSet<_>>();
    if actual_ids != expected_ids {
        failures.push(format!(
            "{}: answer ids were {actual_ids:?}, expected {expected_ids:?}",
            spec.text_id()
        ));
    }

    if let Err(error) = response.validate_against(request) {
        failures.push(format!("{}: {error}", spec.text_id()));
    }
    if response.usage.input_tokens == 0 {
        failures.push(format!("{}: input token usage was zero", spec.text_id()));
    }

    failures
}

#[tokio::test]
async fn reports_identity_mismatches() {
    let request = request();
    let mut response = valid_response(&request).await;
    response.provider = "other".to_owned();
    response.model_id = "other-model".to_owned();
    response.resolved_model_id = " ".to_owned();

    let failures = validation_failures(&spec(), &request, &response);

    assert_eq!(failures.len(), 3);
    assert!(failures.iter().any(|failure| failure.contains("provider")));
    assert!(failures.iter().any(|failure| failure.contains("resolved")));
}

#[tokio::test]
async fn rejects_an_extra_answer_id() {
    let request = request();
    let mut response = valid_response(&request).await;
    response.answers.insert(
        "extra".to_owned(),
        JudgmentAnswer::Condition { probability: 1.0 },
    );

    let failures = validation_failures(&spec(), &request, &response);

    assert_eq!(failures.len(), 1);
    assert!(failures[0].contains("answer ids"));
}

#[tokio::test]
async fn reports_a_shared_validator_failure() {
    let request = request();
    let mut response = valid_response(&request).await;
    response.answers.insert(
        "urgent".to_owned(),
        JudgmentAnswer::Condition { probability: 2.0 },
    );

    let failures = validation_failures(&spec(), &request, &response);

    assert_eq!(failures.len(), 1);
    assert!(failures[0].contains("invalid answer `urgent`"));
}

#[tokio::test]
async fn rejects_zero_input_token_usage() {
    let request = request();
    let mut response = valid_response(&request).await;
    response.usage.input_tokens = 0;

    let failures = validation_failures(&spec(), &request, &response);

    assert_eq!(failures.len(), 1);
    assert!(failures[0].contains("input token usage was zero"));
}

#[tokio::test]
async fn accepts_patched_mock_judgment_output() {
    let request = request();
    let response = valid_response(&request).await;

    assert!(validation_failures(&spec(), &request, &response).is_empty());
}

fn request() -> JudgmentRequest {
    JudgmentRequest {
        state: JudgmentContent::from("Please help with my account today."),
        questions: BTreeMap::from([
            (
                "urgent".to_owned(),
                JudgmentQuestion::condition("Is the message urgent?", None),
            ),
            (
                "queue".to_owned(),
                JudgmentQuestion::choice(
                    "Choose a queue",
                    [("billing", None), ("technical", None), ("other", None)],
                ),
            ),
            (
                "frustration".to_owned(),
                JudgmentQuestion::score(
                    "Rate frustration",
                    ["Calm", "Frustrated", "Very frustrated"],
                ),
            ),
        ]),
    }
}

async fn valid_response(request: &JudgmentRequest) -> JudgmentResponse {
    let mut response = MockJudgmentModel
        .judge(request)
        .await
        .expect("built-in judgment mock should answer a valid request");
    response.provider = "typesafe".to_owned();
    response.model_id = "jev-latest".to_owned();
    response.resolved_model_id = "jev-1.13.0".to_owned();
    response.usage.input_tokens = 1;
    response.usage.total_tokens = 1;
    response
}

fn spec() -> ModelSpec {
    ModelSpec {
        features: Cow::Borrowed(JUDGMENT_FEATURES),
        ..ai_models_typesafe::JEV_LATEST
    }
}
