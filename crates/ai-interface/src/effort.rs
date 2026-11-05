//! Portable reasoning effort and supported-effort sets.

use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
/// Portable effort ordered from disabled thinking to maximum thinking.
pub enum Effort {
    /// Turn thinking off with the provider's explicit off control.
    Disabled,
    /// Use the lowest thinking effort.
    Low,
    /// Use balanced thinking effort.
    Medium,
    /// Use high thinking effort.
    High,
    /// Use stronger-than-high thinking effort.
    ExtraHigh,
    /// Use the largest thinking effort.
    Max,
}

impl Effort {
    /// Every portable effort, in increasing order.
    pub const ALL: [Self; 6] = [
        Self::Disabled,
        Self::Low,
        Self::Medium,
        Self::High,
        Self::ExtraHigh,
        Self::Max,
    ];

    /// Returns the stable value used in logs and documentation.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Disabled => "disabled",
            Self::Low => "low",
            Self::Medium => "medium",
            Self::High => "high",
            Self::ExtraHigh => "extra_high",
            Self::Max => "max",
        }
    }

    /// Returns whether the effort enables thinking.
    pub const fn is_enabled(self) -> bool {
        !matches!(self, Self::Disabled)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
/// An ordered set of explicitly supported efforts, which can be empty.
pub struct EffortSet(u8);

impl EffortSet {
    /// Builds a set with no explicit effort controls.
    pub const fn empty() -> Self {
        Self(0)
    }

    /// Builds a set with one supported effort.
    pub const fn new(effort: Effort) -> Self {
        Self(1 << effort as u8)
    }

    /// Adds a supported effort without changing the set's ordering.
    pub const fn with(self, effort: Effort) -> Self {
        Self(self.0 | (1 << effort as u8))
    }

    /// Returns whether this set supports the effort.
    pub const fn contains(self, effort: Effort) -> bool {
        self.0 & (1 << effort as u8) != 0
    }

    /// Returns each supported effort once, in increasing order.
    pub fn iter(self) -> impl DoubleEndedIterator<Item = Effort> + Clone {
        Effort::ALL
            .into_iter()
            .filter(move |effort| self.contains(*effort))
    }
}

#[cfg(test)]
#[path = "_tests_/effort_tests.rs"]
mod effort_tests;
