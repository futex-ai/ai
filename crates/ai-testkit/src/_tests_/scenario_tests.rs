//! Model-request construction and provider-context retention checks.

use ai_interface::{
    ConversationMessage, FinishReason, ModelResponse, ProviderConversationItem, ToolCall,
};
use serde_json::json;

use crate::scenario::Scenario;

#[test]
fn tool_follow_up_preserves_assistant_context_and_matches_tool_results() {
    let context = vec![ProviderConversationItem::OpenAiReasoning {
        id: "rs_1".to_owned(),
        summary: vec![],
        encrypted_content: Some("opaque-state".to_owned()),
    }];
    let calls = vec![ToolCall {
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
        assistant_message: "Checking now".to_owned(),
        tool_calls: calls.clone(),
        finish_reason: FinishReason::ToolCalls,
        structured_output: None,
        provider_context: context.clone(),
        usage: Default::default(),
        attempts: Vec::new(),
    };
    let request = Scenario::ToolLoop
        .follow_up(&response)
        .expect("tool request");
    assert_eq!(
        request.messages[1],
        ConversationMessage::assistant_with_provider_context("Checking now", calls, context)
    );
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
        image.messages[0].content_parts[1],
        ai_interface::ConversationContentPart::Image { .. }
    ));
}
