//! Settings and invocation identity shared by the engine stages.

use ai_models_core::ThinkingLevel;

pub(crate) struct Dialect {
    pub(crate) provider: &'static str,
    pub(crate) endpoint: &'static str,
}

#[derive(Clone, Copy)]
pub(crate) struct CompletionContext<'a> {
    pub(crate) dialect: &'a Dialect,
    pub(crate) catalog_model_id: &'a str,
    pub(crate) provider_model_id: &'a str,
    pub(crate) thinking_level: ThinkingLevel,
}
