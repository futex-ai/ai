//! Typed questions submitted to judgment models.

use std::collections::BTreeMap;

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
        /// Option labels and their optional descriptions.
        options: BTreeMap<String, Option<JudgmentContent>>,
    },
    /// Places the state on an ordered, zero-indexed rubric.
    Score {
        /// Instructions that define how to score the state.
        instructions: JudgmentContent,
        /// Ordered levels and their optional descriptions.
        levels: Vec<Option<JudgmentContent>>,
    },
}

impl JudgmentQuestion {
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
                .map(|(label, description)| (label.into(), description))
                .collect(),
        }
    }

    /// Builds a score question without validating it.
    pub fn score(
        instructions: impl Into<JudgmentContent>,
        levels: impl IntoIterator<Item = Option<JudgmentContent>>,
    ) -> Self {
        Self::Score {
            instructions: instructions.into(),
            levels: levels.into_iter().collect(),
        }
    }
}
