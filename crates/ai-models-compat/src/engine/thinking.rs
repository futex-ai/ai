//! Thinking-parameter wire forms for a resolved request effort.

use ai_interface::{Effort, ModelControl, ModelError, ModelResult};

use super::{
    dialect::{Dialect, ThinkingFormat},
    request_types::ChatCompletionsThinking,
};

pub(super) fn thinking_fields(
    dialect: &Dialect,
    model_id: &str,
    effort: Option<Effort>,
) -> ModelResult<(Option<ChatCompletionsThinking>, Option<String>)> {
    let Some(effort) = effort else {
        return Ok((None, None));
    };
    Ok(match dialect.thinking {
        ThinkingFormat::GrokEffort { fixed_prefix } => {
            let wire_effort = match effort {
                Effort::Disabled => Some("none"),
                _ if model_id.starts_with(fixed_prefix) => None,
                Effort::Low => Some("low"),
                Effort::Medium => Some("medium"),
                Effort::High | Effort::Max => Some("high"),
                Effort::ExtraHigh => Some("xhigh"),
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
        ThinkingFormat::Effort if effort == Effort::Disabled => {
            return Err(ModelError::unsupported_control(
                dialect.provider,
                model_id,
                ModelControl::Effort,
            ));
        }
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
    })
}
