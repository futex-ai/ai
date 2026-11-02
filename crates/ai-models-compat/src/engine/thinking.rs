//! Thinking-parameter wire forms for a resolved request effort.

use ai_interface::Effort;

use super::{
    dialect::{Dialect, ThinkingFormat},
    request_types::ChatCompletionsThinking,
};

pub(super) fn thinking_fields(
    dialect: &Dialect,
    model_id: &str,
    effort: Effort,
) -> (Option<ChatCompletionsThinking>, Option<String>) {
    match dialect.thinking {
        ThinkingFormat::GrokEffort { fixed_prefix } => {
            let wire_effort = if model_id.starts_with(fixed_prefix) {
                None
            } else {
                match effort {
                    Effort::Disabled => None,
                    Effort::Low => Some("low"),
                    Effort::Medium => Some("medium"),
                    Effort::High | Effort::Max => Some("high"),
                    Effort::ExtraHigh => Some("xhigh"),
                }
            };
            (None, wire_effort.map(str::to_owned))
        }
        ThinkingFormat::Adaptive => (
            Some(ChatCompletionsThinking {
                kind: if effort.is_enabled() {
                    "adaptive"
                } else {
                    "disabled"
                },
            }),
            None,
        ),
        ThinkingFormat::Flags => (None, None),
        ThinkingFormat::Effort => (None, Some(effort.as_str().to_owned())),
        ThinkingFormat::ToggleEffort => (
            Some(ChatCompletionsThinking {
                kind: if effort.is_enabled() {
                    "enabled"
                } else {
                    "disabled"
                },
            }),
            effort.is_enabled().then(|| effort.as_str().to_owned()),
        ),
    }
}
