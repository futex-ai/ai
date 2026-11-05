//! Google function-calling configuration mapping.

use ai_models_core::ResolvedToolChoice;
use serde::Serialize;

#[derive(Debug, Serialize)]
pub(super) struct GoogleToolConfig {
    #[serde(rename = "functionCallingConfig")]
    function_calling_config: GoogleFunctionCallingConfig,
}

#[derive(Debug, Serialize)]
struct GoogleFunctionCallingConfig {
    mode: String,
    #[serde(skip_serializing_if = "Vec::is_empty", rename = "allowedFunctionNames")]
    allowed_function_names: Vec<String>,
}

pub(super) fn tool_config(choice: Option<&ResolvedToolChoice>) -> Option<GoogleToolConfig> {
    let (mode, allowed_function_names) = match choice {
        Some(ResolvedToolChoice::None) => ("NONE", Vec::new()),
        Some(ResolvedToolChoice::Auto) => ("AUTO", Vec::new()),
        Some(ResolvedToolChoice::Required) => ("ANY", Vec::new()),
        Some(ResolvedToolChoice::Function(name)) => ("ANY", vec![name.clone()]),
        None => return None,
    };
    Some(GoogleToolConfig {
        function_calling_config: GoogleFunctionCallingConfig {
            mode: mode.to_owned(),
            allowed_function_names,
        },
    })
}
