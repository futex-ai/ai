//! Typed questions submitted to judgment models.

use serde::{Deserialize, Serialize};

use super::JudgmentContent;

/// Optional descriptions for the yes and no outcomes of a condition.
#[derive(Clone, Debug, Default, Deserialize, PartialEq, Serialize)]
pub struct JudgmentConditionCriteria {
    /// Description of evidence that supports the condition.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub yes: Option<JudgmentContent>,
    /// Description of evidence that does not support the condition.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub no: Option<JudgmentContent>,
}

impl JudgmentConditionCriteria {
    /// Builds condition criteria from optional yes and no descriptions.
    pub fn new(yes: Option<JudgmentContent>, no: Option<JudgmentContent>) -> Self {
        Self { yes, no }
    }
}

/// One choice option with a label and an optional description.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct JudgmentOption {
    /// Label that identifies this option in the answer.
    pub label: String,
    /// Optional description that helps the model choose.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<JudgmentContent>,
}

impl JudgmentOption {
    /// Builds an option from a label and an optional description.
    pub fn new(label: impl Into<String>, description: Option<JudgmentContent>) -> Self {
        Self {
            label: label.into(),
            description,
        }
    }
}

/// One score level with a label and an optional description.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct JudgmentLevel {
    /// Label shown to the model for this level.
    pub label: String,
    /// Optional description that explains the level.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<JudgmentContent>,
}

impl JudgmentLevel {
    /// Builds a level from a label and an optional description.
    pub fn new(label: impl Into<String>, description: Option<JudgmentContent>) -> Self {
        Self {
            label: label.into(),
            description,
        }
    }
}

impl From<&str> for JudgmentLevel {
    fn from(label: &str) -> Self {
        Self::new(label, None)
    }
}

impl From<String> for JudgmentLevel {
    fn from(label: String) -> Self {
        Self::new(label, None)
    }
}

/// Shape shared by a judgment question and its answer.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum JudgmentQuestionKind {
    /// A binary condition probability.
    Condition,
    /// A selection from labeled options.
    Choice,
    /// A position on an ordered rubric.
    Score,
}

/// A typed question evaluated against shared request state.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum JudgmentQuestion {
    /// Determines the probability that a condition holds.
    Condition {
        /// Instructions that define the condition.
        instructions: JudgmentContent,
        /// Optional descriptions of the yes and no outcomes.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        criteria: Option<JudgmentConditionCriteria>,
    },
    /// Selects one label from a caller-defined set of options.
    Choice {
        /// Instructions that define how to choose an option.
        instructions: JudgmentContent,
        /// Options in the order supplied by the caller.
        options: Vec<JudgmentOption>,
    },
    /// Places the state on an ordered, zero-indexed rubric.
    Score {
        /// Instructions that define how to score the state.
        instructions: JudgmentContent,
        /// Levels in ascending score order.
        levels: Vec<JudgmentLevel>,
    },
}

impl JudgmentQuestion {
    /// Returns the answer shape required by this question.
    pub fn kind(&self) -> JudgmentQuestionKind {
        match self {
            Self::Condition { .. } => JudgmentQuestionKind::Condition,
            Self::Choice { .. } => JudgmentQuestionKind::Choice,
            Self::Score { .. } => JudgmentQuestionKind::Score,
        }
    }

    /// Builds a condition question without validating it.
    pub fn condition(
        instructions: impl Into<JudgmentContent>,
        criteria: Option<JudgmentConditionCriteria>,
    ) -> Self {
        Self::Condition {
            instructions: instructions.into(),
            criteria,
        }
    }

    /// Builds a choice question without validating it.
    pub fn choice(
        instructions: impl Into<JudgmentContent>,
        options: impl IntoIterator<Item = (impl Into<String>, Option<JudgmentContent>)>,
    ) -> Self {
        Self::Choice {
            instructions: instructions.into(),
            options: options
                .into_iter()
                .map(|(label, description)| JudgmentOption::new(label, description))
                .collect(),
        }
    }

    /// Builds a score question without validating it.
    pub fn score(
        instructions: impl Into<JudgmentContent>,
        levels: impl IntoIterator<Item = impl Into<JudgmentLevel>>,
    ) -> Self {
        Self::Score {
            instructions: instructions.into(),
            levels: levels.into_iter().map(Into::into).collect(),
        }
    }
}

#[cfg(test)]
#[path = "_tests_/question_tests.rs"]
mod question_tests;
