//! Normalized answers returned by judgment models.

use std::collections::BTreeMap;

use serde::de::Error as DeError;
use serde::{Deserialize, Deserializer, Serialize};

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

/// Parses decimal string level keys into indexes.
///
/// The internally tagged enum buffers map keys as strings before this field
/// is visited, so the derived `u32` key deserializer cannot be used directly.
fn deserialize_score_probabilities<'de, D>(deserializer: D) -> Result<BTreeMap<u32, f64>, D::Error>
where
    D: Deserializer<'de>,
{
    let encoded = BTreeMap::<String, f64>::deserialize(deserializer)?;
    let mut probabilities = BTreeMap::new();
    for (key, probability) in encoded {
        let index = match key.parse::<u32>() {
            Ok(index) => index,
            Err(source) => {
                return Err(D::Error::custom(format!(
                    "score level key `{key}` is not a decimal index: {source}"
                )));
            }
        };
        probabilities.insert(index, probability);
    }
    Ok(probabilities)
}
