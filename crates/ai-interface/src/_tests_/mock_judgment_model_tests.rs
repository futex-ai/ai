//! Built-in mock judgment model tests.

use std::collections::BTreeMap;
use std::sync::Arc;

use unimock::{MockFn, Unimock, matching};

use crate::{
    DynJudgmentModel, JudgmentAnswer, JudgmentContent, JudgmentError, JudgmentModel,
    JudgmentModelMock, JudgmentQuestion, JudgmentRequest, JudgmentResponse, MockJudgmentModel,
    ModelUsage,
};

#[tokio::test]
async fn mock_returns_deterministic_answers_and_identity() {
    let model: DynJudgmentModel = Arc::new(MockJudgmentModel);
    let request = three_kind_request();

    let response = model.judge(&request).await.unwrap();

    assert_eq!(response.provider, "mock");
    assert_eq!(response.model_id, "mock-judgment");
    assert_eq!(response.resolved_model_id, "mock-judgment");
    assert_eq!(response.usage, ModelUsage::default());
    assert_eq!(
        response.answers,
        BTreeMap::from([
            (
                "choice".to_owned(),
                JudgmentAnswer::Choice {
                    selected: "billing".to_owned(),
                    probabilities: BTreeMap::from([
                        ("billing".to_owned(), 1.0),
                        ("technical".to_owned(), 0.0),
                    ]),
                    confidence: 1.0,
                },
            ),
            (
                "condition".to_owned(),
                JudgmentAnswer::Condition { probability: 1.0 },
            ),
            (
                "score".to_owned(),
                JudgmentAnswer::Score {
                    expected: 0.0,
                    probabilities: BTreeMap::from([(0, 1.0), (1, 0.0), (2, 0.0)]),
                    confidence: 1.0,
                },
            ),
        ])
    );
    response.validate_against(&request).unwrap();
    assert_eq!(model.judge(&request).await.unwrap(), response);
}

#[tokio::test]
async fn mock_applies_shared_request_validation() {
    let error = MockJudgmentModel
        .judge(&JudgmentRequest {
            state: JudgmentContent::from(" \n"),
            questions: BTreeMap::from([(
                "condition".to_owned(),
                JudgmentQuestion::condition("Is it urgent?", None),
            )]),
        })
        .await
        .unwrap_err();

    assert!(matches!(error, JudgmentError::EmptyState));
}

#[tokio::test]
async fn generated_model_mock_returns_a_canned_response() {
    let expected = canned_response();
    let model = Unimock::new(
        JudgmentModelMock::judge
            .next_call(matching!(_))
            .returns(Ok(expected.clone())),
    );

    assert_eq!(model.judge(&three_kind_request()).await.unwrap(), expected);
}

#[tokio::test]
async fn generated_model_mock_returns_a_canned_error() {
    let model = Unimock::new(
        JudgmentModelMock::judge
            .next_call(matching!(_))
            .returns(Err(JudgmentError::NoQuestions)),
    );

    let error = model
        .judge(&three_kind_request())
        .await
        .expect_err("configured error should be returned");

    assert!(matches!(error, JudgmentError::NoQuestions));
}

fn three_kind_request() -> JudgmentRequest {
    JudgmentRequest {
        state: JudgmentContent::from("A support ticket"),
        questions: BTreeMap::from([
            (
                "condition".to_owned(),
                JudgmentQuestion::condition("Is it urgent?", None),
            ),
            (
                "choice".to_owned(),
                JudgmentQuestion::choice("Choose a team", [("technical", None), ("billing", None)]),
            ),
            (
                "score".to_owned(),
                JudgmentQuestion::score("Rate it", [None, None, None]),
            ),
        ]),
    }
}

fn canned_response() -> JudgmentResponse {
    JudgmentResponse {
        provider: "fixture".to_owned(),
        model_id: "fixture-model".to_owned(),
        resolved_model_id: "fixture-model".to_owned(),
        answers: BTreeMap::new(),
        usage: ModelUsage::default(),
    }
}
