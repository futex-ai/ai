//! Judgment request DTO and local validation.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use super::{
    JudgmentContent, JudgmentError, JudgmentQuestion, JudgmentQuestionProblem, JudgmentResult,
};

/// State and typed questions submitted in one judgment call.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct JudgmentRequest {
    /// Text or structured state evaluated by every question.
    pub state: JudgmentContent,
    /// Caller-defined ids mapped to independent questions.
    pub questions: BTreeMap<String, JudgmentQuestion>,
}

impl JudgmentRequest {
    /// Validates state and question structure without provider access.
    pub fn validate(&self) -> JudgmentResult<()> {
        if state_is_empty(&self.state) {
            return Err(JudgmentError::EmptyState);
        }
        if self.questions.is_empty() {
            return Err(JudgmentError::NoQuestions);
        }
        for (id, question) in &self.questions {
            if id.trim().is_empty() {
                return Err(JudgmentError::invalid_question(
                    id,
                    JudgmentQuestionProblem::BlankId,
                ));
            }
            match question {
                JudgmentQuestion::Choice { options, .. } if options.is_empty() => {
                    return Err(JudgmentError::invalid_question(
                        id,
                        JudgmentQuestionProblem::NoOptions,
                    ));
                }
                JudgmentQuestion::Choice { options, .. }
                    if options.keys().any(|label| label.trim().is_empty()) =>
                {
                    return Err(JudgmentError::invalid_question(
                        id,
                        JudgmentQuestionProblem::BlankOptionLabel,
                    ));
                }
                JudgmentQuestion::Score { levels, .. } if levels.len() < 2 => {
                    return Err(JudgmentError::invalid_question(
                        id,
                        JudgmentQuestionProblem::TooFewLevels {
                            levels: levels.len(),
                        },
                    ));
                }
                JudgmentQuestion::Condition { .. }
                | JudgmentQuestion::Choice { .. }
                | JudgmentQuestion::Score { .. } => {}
            }
        }
        Ok(())
    }
}

fn state_is_empty(state: &JudgmentContent) -> bool {
    match state {
        JudgmentContent::Text(text) => text.trim().is_empty(),
        JudgmentContent::Object(object) => object.is_empty(),
        JudgmentContent::Array(array) => array.is_empty(),
    }
}
