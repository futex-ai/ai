//! Model-request construction and provider-context retention checks.

use ai_interface::{
    ConversationMessage, ConversationPart, FinishReason, ModelResponse, ProviderId, ProviderState,
    ToolCall,
};
use serde_json::json;

use crate::scenario::Scenario;

#[test]
fn tool_follow_up_preserves_assistant_context_and_matches_tool_results() {
    let reasoning = ConversationPart::reasoning("").with_provider_state(ProviderState {
        provider: ProviderId::new("openai"),
        model: "test-model".to_owned(),
        version: 1,
        payload: json!({"id": "rs_1", "encrypted_content": "opaque-state"})
            .as_object()
            .unwrap()
            .clone(),
    });
    let calls = [ToolCall {
        id: "call_1".to_owned(),
        name: "lookup".to_owned(),
        input: json!({"id": 7}),
        operation_id: None,
    }];
    let response = ModelResponse {
        provider: "openai".to_owned(),
        model_id: "test-model".to_owned(),
        catalog_model_id: None,
        thinking_level: None,
        parts: vec![
            reasoning.clone(),
            ConversationPart::text("Checking now"),
            ConversationPart::from_tool_call(calls[0].clone()),
        ],
        finish_reason: FinishReason::ToolCalls,
        structured_output: None,
        usage: Default::default(),
        attempts: Vec::new(),
    };
    let request = Scenario::ToolLoop
        .follow_up(&response)
        .expect("tool request");
    assert_eq!(request.messages[1], response.to_assistant_message());
    assert_eq!(
        request.messages[2],
        ConversationMessage::tool("{\"value\":7}", "lookup", "call_1")
    );
    assert_eq!(
        request.messages[3],
        ConversationMessage::user("Use the tool result. Reply with exactly Done.")
    );
}

#[test]
fn schema_and_image_scenarios_set_typed_request_fields() {
    let schema = Scenario::SchemaCall
        .request()
        .response_schema
        .expect("schema request");
    assert_eq!(schema.schema["required"], json!(["ok"]));
    let image = Scenario::ImageInput.request();
    assert!(matches!(
        image.messages[0].parts[1],
        ConversationPart::Image { .. }
    ));
}
