//! Judgment request DTO and local validation.

use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

use super::{
    JudgmentContent, JudgmentError, JudgmentOption, JudgmentQuestion, JudgmentQuestionProblem,
    JudgmentResult,
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
            return Err(JudgmentError::empty_state());
        }
        if self.questions.is_empty() {
            return Err(JudgmentError::no_questions());
        }
        for (id, question) in &self.questions {
            if id.trim().is_empty() {
                return Err(JudgmentError::invalid_question(
                    id,
                    JudgmentQuestionProblem::BlankId,
                ));
            }
            let problem = match question {
                JudgmentQuestion::Choice { options, .. } => choice_problem(options),
                JudgmentQuestion::Score { levels, .. } if levels.len() < 2 => {
                    Some(JudgmentQuestionProblem::TooFewLevels {
                        levels: levels.len(),
                    })
                }
                JudgmentQuestion::Condition { .. } | JudgmentQuestion::Score { .. } => None,
            };
            if let Some(problem) = problem {
                return Err(JudgmentError::invalid_question(id, problem));
            }
        }
        Ok(())
    }
}

fn choice_problem(options: &[JudgmentOption]) -> Option<JudgmentQuestionProblem> {
    if options.is_empty() {
        return Some(JudgmentQuestionProblem::NoOptions);
    }
    if options.iter().any(|option| option.label.trim().is_empty()) {
        return Some(JudgmentQuestionProblem::BlankOptionLabel);
    }

    let mut labels = BTreeSet::new();
    options
        .iter()
        .any(|option| !labels.insert(option.label.as_str()))
        .then_some(JudgmentQuestionProblem::DuplicateOptionLabel)
}

fn state_is_empty(state: &JudgmentContent) -> bool {
    match state {
        JudgmentContent::Text(text) => text.trim().is_empty(),
        JudgmentContent::Object(object) => object.is_empty(),
        JudgmentContent::Array(array) => array.is_empty(),
    }
}

#[cfg(test)]
#[path = "_tests_/request_tests.rs"]
mod request_tests;
