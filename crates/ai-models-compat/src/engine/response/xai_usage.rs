//! Bounded usage wire fields retained by the xAI dialect.

use serde::Deserialize;

#[derive(Clone, Debug, Default, Deserialize)]
pub(super) struct XaiUsage {
    #[serde(default)]
    pub(super) prompt_tokens: u32,
    #[serde(default)]
    pub(super) completion_tokens: u32,
    #[serde(default)]
    pub(super) total_tokens: Option<u32>,
    #[serde(default)]
    pub(super) prompt_tokens_details: XaiPromptDetails,
    #[serde(default)]
    pub(super) completion_tokens_details: XaiCompletionDetails,
}

#[derive(Clone, Debug, Default, Deserialize)]
pub(super) struct XaiPromptDetails {
    #[serde(default)]
    pub(super) cached_tokens: u32,
}

#[derive(Clone, Debug, Default, Deserialize)]
pub(super) struct XaiCompletionDetails {
    #[serde(default)]
    pub(super) reasoning_tokens: u32,
}
