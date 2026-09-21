//! Normalized answers returned by judgment models.

use std::collections::BTreeMap;
use std::fmt;

use serde::de::{Error as DeError, MapAccess, Unexpected, Visitor};
use serde::{Deserialize, Deserializer, Serialize};

use super::JudgmentQuestionKind;

/// A typed answer to one judgment question.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum JudgmentAnswer {
    /// Probability that a condition holds.
    Condition {
        /// Probability of the yes outcome, from zero to one.
        probability: f64,
    },
    /// Selected choice and the complete option distribution.
    Choice {
        /// Label selected by the model.
        selected: String,
        /// Probability for every option label.
        probabilities: BTreeMap<String, f64>,
        /// Provider-reported distribution confidence.
        confidence: f64,
    },
    /// Expected score and the complete level distribution.
    Score {
        /// Expected zero-indexed level value.
        expected: f64,
        /// Probability for every zero-indexed level, keyed by decimal
        /// strings on the wire.
        #[serde(deserialize_with = "deserialize_score_probabilities")]
        probabilities: BTreeMap<u32, f64>,
        /// Provider-reported distribution confidence.
        confidence: f64,
    },
}

impl JudgmentAnswer {
    /// Returns the question shape answered by this value.
    pub fn kind(&self) -> JudgmentQuestionKind {
        match self {
            Self::Condition { .. } => JudgmentQuestionKind::Condition,
            Self::Choice { .. } => JudgmentQuestionKind::Choice,
            Self::Score { .. } => JudgmentQuestionKind::Score,
        }
    }
}

/// Deserializes level-indexed probabilities keyed by canonical decimal strings.
///
/// Internally tagged enums buffer map keys as strings, so this helper parses
/// each key itself, rejecting non-canonical spellings and indexes that appear
/// more than once. Provider crates reuse it for their wire types.
pub fn deserialize_score_probabilities<'de, D>(
    deserializer: D,
) -> Result<BTreeMap<u32, f64>, D::Error>
where
    D: Deserializer<'de>,
{
    deserializer.deserialize_map(ScoreProbabilitiesVisitor)
}

/// Parses canonical level keys without collapsing repeated map entries.
struct ScoreProbabilitiesVisitor;

impl<'de> Visitor<'de> for ScoreProbabilitiesVisitor {
    type Value = BTreeMap<u32, f64>;

    fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("a map keyed by canonical unsigned decimal level indexes")
    }

    fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
    where
        A: MapAccess<'de>,
    {
        let mut probabilities = BTreeMap::new();
        while let Some(key) = map.next_key::<String>()? {
            let index = parse_level_index::<A::Error>(&key)?;
            if probabilities.contains_key(&index) {
                return Err(A::Error::invalid_value(
                    Unexpected::Str(&key),
                    &"a unique level index",
                ));
            }
            let probability = map.next_value::<f64>()?;
            probabilities.insert(index, probability);
        }
        Ok(probabilities)
    }
}

fn parse_level_index<E>(key: &str) -> Result<u32, E>
where
    E: DeError,
{
    let is_canonical = key == "0"
        || (!key.is_empty()
            && !key.starts_with('0')
            && key.bytes().all(|byte| byte.is_ascii_digit()));
    if is_canonical && let Ok(index) = key.parse::<u32>() {
        return Ok(index);
    }
    Err(E::invalid_value(
        Unexpected::Str(key),
        &"a canonical unsigned decimal level index",
    ))
}

#[cfg(test)]
#[path = "_tests_/answer_tests.rs"]
mod answer_tests;
