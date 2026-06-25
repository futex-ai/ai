use crate::{OpenAiReasoningSummary, ProviderConversationItem};
use serde_json::json;

#[test]
fn openai_reasoning_context_serializes_with_provider_tag() {
    let item = ProviderConversationItem::OpenAiReasoning {
        id: "rs_123".to_owned(),
        summary: vec![OpenAiReasoningSummary {
            kind: "summary_text".to_owned(),
            text: "Used the retained plan.".to_owned(),
        }],
        encrypted_content: Some("encrypted".to_owned()),
    };

    assert_eq!(
        serde_json::to_value(item).unwrap(),
        json!({
            "type": "openai_reasoning",
            "id": "rs_123",
            "summary": [{
                "type": "summary_text",
                "text": "Used the retained plan."
            }],
            "encrypted_content": "encrypted"
        })
    );
}
