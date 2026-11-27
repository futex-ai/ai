//! Judgment request DTO and local validation.

use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

use super::{
    JudgmentError, JudgmentInput, JudgmentOption, JudgmentQuestion, JudgmentQuestionProblem,
    JudgmentResult,
};

/// Input and typed questions submitted in one judgment call.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct JudgmentRequest {
    /// Content or a conversation evaluated by every question.
    pub input: JudgmentInput,
    /// Caller-defined ids mapped to independent questions.
    pub questions: BTreeMap<String, JudgmentQuestion>,
}

impl JudgmentRequest {
    /// Validates input and question structure without provider access.
    pub fn validate(&self) -> JudgmentResult<()> {
        self.input.validate()?;
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
            if question.instructions().parts.is_empty() {
                return Err(JudgmentError::invalid_question(
                    id,
                    JudgmentQuestionProblem::NoInstructions,
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

#[cfg(test)]
#[path = "_tests_/request_tests.rs"]
mod request_tests;
