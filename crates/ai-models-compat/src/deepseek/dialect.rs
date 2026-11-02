//! DeepSeek endpoint and dialect settings.

use crate::engine::Dialect;

pub(super) static DIALECT: Dialect = Dialect {
    provider: "deepseek",
    endpoint: "https://api.deepseek.com/chat/completions",
};
