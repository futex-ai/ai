//! Data that controls portable generation behavior for a model.

use crate::{Effort, EffortSet};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
/// Conditions under which a model can force tool selection.
pub enum ForcedToolChoice {
    /// Allow both required and named-function choices at every effort.
    Always,
    /// Allow forced choices only while thinking is disabled.
    OnlyWithoutThinking,
    /// Allow required tool use but never a named-function choice.
    RequiredOnly,
    /// Never allow forced tool selection.
    Never,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
/// Whether a model accepts caller-selected sampling values.
pub enum Sampling {
    /// Accept temperature and top-p at every effort.
    Adjustable,
    /// Omit temperature and top-p while thinking is enabled.
    FixedWhileThinking,
    /// Always omit temperature and top-p.
    Fixed,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
/// Provider-owned defaults for models that no catalog lists.
pub struct CustomModelDefaults {
    /// Documented total context tokens, or zero when unknown.
    pub context_window_tokens: u32,
    /// Maximum allowed generated-token limit.
    pub max_output_tokens: u32,
    /// Nonempty ordered set of supported efforts.
    pub efforts: EffortSet,
    /// Effort used when a request supplies none.
    pub default_effort: Effort,
    /// Conditions under which forced tool choices are allowed.
    pub forced_tool_choice: ForcedToolChoice,
    /// Conditions under which sampling values are adjustable.
    pub sampling: Sampling,
    /// Whether the model accepts nonempty stop sequences.
    pub supports_stop_sequences: bool,
    /// Whether the adapter supports a deferred lifecycle.
    pub supports_deferred: bool,
    /// Whether every provider request must include an output-token limit.
    pub requires_output_limit: bool,
    /// Efforts whose tool continuations need private provider replay state.
    pub replay_efforts: &'static [Effort],
}
