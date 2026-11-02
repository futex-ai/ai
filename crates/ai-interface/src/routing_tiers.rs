//! Provider model scores used by routing preferences.

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
/// Coarse model speed tier used as a routing preference.
pub enum SpeedTier {
    /// Slowest configured routing tier.
    Slow,
    /// Default balanced speed tier.
    Medium,
    /// Faster than balanced models.
    Fast,
    /// Fastest configured routing tier.
    VeryFast,
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
/// Coarse model cost tier used as a routing preference.
pub enum CostTier {
    /// Lowest-cost configured routing tier.
    Low,
    /// Balanced cost tier.
    Medium,
    /// Higher-cost configured routing tier.
    High,
    /// Highest-cost configured routing tier.
    Premium,
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
/// Validated model intelligence ranking from one to ten.
pub enum IntelligenceScore {
    /// Intelligence score 1.
    One = 1,
    /// Intelligence score 2.
    Two = 2,
    /// Intelligence score 3.
    Three = 3,
    /// Intelligence score 4.
    Four = 4,
    /// Intelligence score 5.
    Five = 5,
    /// Intelligence score 6.
    Six = 6,
    /// Intelligence score 7.
    Seven = 7,
    /// Intelligence score 8.
    Eight = 8,
    /// Intelligence score 9.
    Nine = 9,
    /// Intelligence score 10.
    Ten = 10,
}

impl IntelligenceScore {
    /// Returns the numeric score represented by this validated value.
    pub fn value(self) -> u8 {
        self as u8
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
/// Routing metadata at the model's default effort.
pub struct RoutingTiers {
    /// Model intelligence ranking from one to ten.
    pub intelligence: IntelligenceScore,
    /// Coarse model speed tier.
    pub speed: SpeedTier,
    /// Coarse model cost tier.
    pub cost: CostTier,
}
