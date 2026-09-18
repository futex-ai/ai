//! Built-in mock judgment model for development and tests.

use std::collections::BTreeMap;

use async_trait::async_trait;

use crate::{
    JudgmentAnswer, JudgmentError, JudgmentModel, JudgmentQuestion, JudgmentQuestionProblem,
    JudgmentRequest, JudgmentResponse, JudgmentResult, ModelUsage,
};

const MOCK_MODEL_ID: &str = "mock-judgment";

/// Deterministic mock judgment model used by development and tests.
#[derive(Clone, Debug, Default)]
pub struct MockJudgmentModel;

#[async_trait]
impl JudgmentModel for MockJudgmentModel {
    async fn judge(&self, request: &JudgmentRequest) -> JudgmentResult<JudgmentResponse> {
        request.validate()?;
        let mut answers = BTreeMap::new();
        for (id, question) in &request.questions {
            answers.insert(id.clone(), answer_question(id, question)?);
        }
        Ok(JudgmentResponse {
            provider: "mock".to_owned(),
            model_id: MOCK_MODEL_ID.to_owned(),
            resolved_model_id: MOCK_MODEL_ID.to_owned(),
            answers,
            usage: ModelUsage::default(),
        })
    }
}

fn answer_question(id: &str, question: &JudgmentQuestion) -> JudgmentResult<JudgmentAnswer> {
    match question {
        JudgmentQuestion::Condition { .. } => Ok(JudgmentAnswer::Condition { probability: 1.0 }),
        JudgmentQuestion::Choice { options, .. } => {
            let Some(selected) = options.keys().next().cloned() else {
                return Err(JudgmentError::invalid_question(
                    id,
                    JudgmentQuestionProblem::NoOptions,
                ));
            };
            let probabilities = options
                .keys()
                .map(|label| (label.clone(), f64::from(label == &selected)))
                .collect();
            Ok(JudgmentAnswer::Choice {
                selected,
                probabilities,
                confidence: 1.0,
            })
        }
        JudgmentQuestion::Score { levels, .. } => {
            let probabilities = levels
                .iter()
                .enumerate()
                .map(|(index, _)| (index as u32, f64::from(index == 0)))
                .collect();
            Ok(JudgmentAnswer::Score {
                expected: 0.0,
                probabilities,
                confidence: 1.0,
            })
        }
    }
}

#[cfg(test)]
#[path = "_tests_/mock_judgment_model_tests.rs"]
mod mock_judgment_model_tests;
