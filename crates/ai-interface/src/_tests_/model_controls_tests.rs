use std::time::Duration;

use serde_json::json;

use crate::{
    ModelCallControls, ModelCompletionMode, ModelControl, ModelExecutionControls,
    ModelGenerationControls, ModelRequest, ModelToolChoice,
};

#[test]
fn default_controls_are_absent_from_serialized_requests() {
    let value = serde_json::to_value(ModelRequest {
        system_prompt: "system".to_owned(),
        messages: Vec::new(),
        tools: Vec::new(),
        response_schema: None,
        controls: ModelCallControls::default(),
    })
    .expect("request should serialize");

    assert_eq!(
        value,
        json!({
            "system_prompt": "system",
            "messages": [],
            "tools": []
        })
    );
}

#[test]
fn system_prompt_blankness_uses_trim_semantics() {
    for (system_prompt, expected) in [
        ("", None),
        (" \t\n", None),
        (" normal instruction ", Some(" normal instruction ")),
    ] {
        let request = ModelRequest {
            system_prompt: system_prompt.to_owned(),
            messages: Vec::new(),
            tools: Vec::new(),
            response_schema: None,
            controls: Default::default(),
        };

        assert_eq!(request.nonblank_system_prompt(), expected);
    }
}

#[test]
fn explicit_controls_round_trip_without_losing_order_or_types() {
    let controls = ModelCallControls {
        generation: ModelGenerationControls {
            temperature: Some(0.2),
            top_p: Some(0.9),
            max_output_tokens: Some(8000),
            stop_sequences: vec!["first".to_owned(), "second".to_owned()],
            tool_choice: Some(ModelToolChoice::Function("lookup".to_owned())),
        },
        execution: ModelExecutionControls {
            total_timeout: Some(Duration::from_secs(600)),
            completion_mode: ModelCompletionMode::PreferDeferred,
        },
    };

    let encoded = serde_json::to_value(&controls).expect("controls should serialize");
    let decoded: ModelCallControls =
        serde_json::from_value(encoded).expect("controls should deserialize");

    assert_eq!(decoded, controls);
}

#[test]
fn required_or_auto_round_trips_as_a_distinct_typed_policy() {
    let choice = ModelToolChoice::RequiredOrAuto;
    let encoded = serde_json::to_value(&choice).expect("tool choice should serialize");
    let decoded: ModelToolChoice =
        serde_json::from_value(encoded).expect("tool choice should deserialize");

    assert_eq!(decoded, choice);
    assert_ne!(decoded, ModelToolChoice::Required);
    assert_ne!(decoded, ModelToolChoice::Auto);
}

#[test]
fn completion_preference_resolves_without_provider_identity() {
    let preferred = ModelExecutionControls {
        completion_mode: ModelCompletionMode::PreferDeferred,
        ..Default::default()
    };
    let required = ModelExecutionControls {
        completion_mode: ModelCompletionMode::RequireDeferred,
        ..Default::default()
    };

    assert_eq!(preferred.resolve_deferred(false), Ok(false));
    assert_eq!(preferred.resolve_deferred(true), Ok(true));
    assert_eq!(required.resolve_deferred(true), Ok(true));
    assert_eq!(
        required.resolve_deferred(false),
        Err(ModelControl::CompletionMode)
    );
}
