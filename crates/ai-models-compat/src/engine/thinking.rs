//! Thinking-parameter wire forms selected by dialect settings.

use ai_interface::{ModelError, ModelResult};
use ai_models_core::ThinkingLevel;

use super::{
    dialect::{Dialect, ThinkingFormat},
    request_types::ChatCompletionsThinking,
};

pub(super) fn thinking_fields(
    dialect: &Dialect,
    model_id: &str,
    level: ThinkingLevel,
) -> ModelResult<(Option<ChatCompletionsThinking>, Option<String>)> {
    match dialect.thinking {
        ThinkingFormat::Effort => Ok((None, Some(level.as_str().to_owned()))),
        ThinkingFormat::ToggleEffort => {
            let (kind, effort) = match level {
                ThinkingLevel::Disabled => ("disabled", None),
                ThinkingLevel::High => ("enabled", Some("high".to_owned())),
                ThinkingLevel::Max => ("enabled", Some("max".to_owned())),
                ThinkingLevel::Low | ThinkingLevel::Medium | ThinkingLevel::ExtraHigh => {
                    return Err(ModelError::provider(
                        dialect.provider,
                        model_id,
                        "unsupported DeepSeek thinking level",
                    ));
                }
            };
            Ok((Some(ChatCompletionsThinking { kind }), effort))
        }
    }
}
