//! Built-in mock judgment model tests.

use std::collections::BTreeMap;

use crate::{
    JudgmentAnswer, JudgmentContent, JudgmentError, JudgmentModel, JudgmentQuestion,
    JudgmentRequest, MockJudgmentModel, ModelUsage,
};

#[tokio::test]
async fn mock_returns_deterministic_answers_and_identity() {
    let model = MockJudgmentModel;
    let request = JudgmentRequest {
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
    };

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
