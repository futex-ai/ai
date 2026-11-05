//! Pure Markdown formatting of model capability facts.

use ai_interface::{Effort, ForcedToolChoice, ModelSpec, Sampling};

pub(super) fn render_matrix(specs: &[ModelSpec]) -> String {
    let mut output = String::from(concat!(
        "Bold marks the default effort or provider default. `default` marks an output limit sent when the request omits one.\n",
        "`u32::MAX` adds no output ceiling beyond the request type. Context 0 means unknown.\n\n",
        "| Text id | Efforts | Max output | Context | Sampling | Stops | Forced tools | Deferred | Replay efforts |\n",
        "| --- | --- | --- | --- | --- | --- | --- | --- | --- |\n",
    ));
    for spec in specs {
        output.push_str(&render_row(spec));
    }
    output
}

fn render_row(spec: &ModelSpec) -> String {
    let mut efforts = spec
        .efforts
        .iter()
        .map(|effort| {
            if Some(effort) == spec.default_effort {
                format!("**{}**", effort.as_str())
            } else {
                effort.as_str().to_owned()
            }
        })
        .collect::<Vec<_>>()
        .join(", ");
    if spec.default_effort.is_none() {
        efforts = if efforts.is_empty() {
            "**provider default**".to_owned()
        } else {
            format!("**provider default**, {efforts}")
        };
    }
    let replay = spec
        .efforts
        .iter()
        .filter(|effort| spec.needs_replay(*effort));
    let replay = effort_list(replay);
    let maximum = if spec.max_output_tokens == u32::MAX {
        "u32::MAX".to_owned()
    } else {
        spec.max_output_tokens.to_string()
    };
    let maximum = if spec.requires_output_limit {
        format!("{maximum} default")
    } else {
        maximum
    };
    let sampling = match spec.sampling {
        Sampling::Adjustable => "adjustable",
        Sampling::FixedWhileThinking => "fixed while thinking",
        Sampling::Fixed => "fixed",
    };
    let forced_tools = match spec.forced_tool_choice {
        ForcedToolChoice::Always => "always",
        ForcedToolChoice::OnlyWithoutThinking => "only without thinking",
        ForcedToolChoice::RequiredOnly => "required only",
        ForcedToolChoice::Never => "never",
    };
    format!(
        "| `{}` | {efforts} | {maximum} | {} | {sampling} | {} | {forced_tools} | {} | {replay} |\n",
        spec.text_id(),
        spec.context_window_tokens,
        yes_no(spec.supports_stop_sequences),
        yes_no(spec.supports_deferred),
    )
}

fn effort_list(efforts: impl Iterator<Item = Effort>) -> String {
    let efforts = efforts.map(Effort::as_str).collect::<Vec<_>>().join(", ");
    if efforts.is_empty() {
        "none".to_owned()
    } else {
        efforts
    }
}

fn yes_no(value: bool) -> &'static str {
    if value { "yes" } else { "no" }
}

#[cfg(test)]
#[path = "_tests_/render_tests.rs"]
mod render_tests;
