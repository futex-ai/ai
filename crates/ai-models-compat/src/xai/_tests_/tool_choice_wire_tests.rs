//! Tool-choice wire forms with and without xAI tools.

use ai_interface::{ConversationMessage, ModelRequest, ModelToolChoice, ToolDefinition};
use ai_models_core::resolve_controls;
use serde_json::{Value, json};

use crate::engine::request::build_request;

use super::{GROK_4_5, dialect::DIALECT};

#[test]
fn omits_none_choice_without_tools() {
    let body = request_json(Some(ModelToolChoice::None), false);

    assert_eq!(body.get("tools"), None);
    assert_eq!(body.get("tool_choice"), None);
}

#[test]
fn omits_auto_choice_without_tools() {
    let body = request_json(Some(ModelToolChoice::Auto), false);

    assert_eq!(body.get("tools"), None);
    assert_eq!(body.get("tool_choice"), None);
}

#[test]
fn keeps_choices_with_tools_and_forced_choices_without_tools() {
    for (case, has_tools, choice, expected) in [
        (
            "none with tools",
            true,
            Some(ModelToolChoice::None),
            json!("none"),
        ),
        (
            "auto with tools",
            true,
            Some(ModelToolChoice::Auto),
            json!("auto"),
        ),
        ("default with tools", true, None, json!("auto")),
        (
            "required without tools",
            false,
            Some(ModelToolChoice::Required),
            json!("required"),
        ),
        (
            "function without tools",
            false,
            Some(ModelToolChoice::Function("lookup".to_owned())),
            json!({"type": "function", "function": {"name": "lookup"}}),
        ),
        (
            "required with tools",
            true,
            Some(ModelToolChoice::Required),
            json!("required"),
        ),
        (
            "function with tools",
            true,
            Some(ModelToolChoice::Function("lookup".to_owned())),
            json!({"type": "function", "function": {"name": "lookup"}}),
        ),
    ] {
        let body = request_json(choice, has_tools);

        assert_eq!(body.get("tool_choice"), Some(&expected), "{case}");
    }
}

fn request_json(choice: Option<ModelToolChoice>, has_tools: bool) -> Value {
    let mut request = ModelRequest {
        system_prompt: String::new(),
        messages: vec![ConversationMessage::user("hello")],
        tools: if has_tools {
            vec![ToolDefinition {
                name: "lookup".to_owned(),
                description: "Look up a value".to_owned(),
                input_schema: json!({"type": "object"}),
                activity_verb: None,
            }]
        } else {
            Vec::new()
        },
        response_schema: None,
        controls: Default::default(),
    };
    request.controls.generation.tool_choice = choice;
    let controls = resolve_controls(&request, &GROK_4_5).expect("xAI controls should resolve");
    let body = build_request(
        &DIALECT,
        GROK_4_5.provider_model_id.as_ref(),
        &request,
        &controls,
    )
    .expect("xAI request should build");
    serde_json::to_value(body).expect("xAI request should serialize")
}
