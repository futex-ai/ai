use serde_json::json;

use crate::{
    ConversationMessage, ErrorKind, MockModel, Model, ModelError, ModelRequest,
    StructuredOutputSchema,
};

#[tokio::test]
async fn mock_model_returns_acknowledged_subject_and_usage() {
    let model = MockModel::new("mock-dev");
    let response = model
        .complete(&ModelRequest {
            system_prompt: "Be concise.".to_owned(),
            messages: vec![ConversationMessage::user("- body: inspect the queue")],
            tools: Vec::new(),
            response_schema: None,
            controls: Default::default(),
        })
        .await
        .expect("mock model should succeed");

    assert_eq!(response.provider, "mock");
    assert_eq!(response.model_id, "mock-dev");
    assert_eq!(response.text(), "Acknowledged: inspect the queue");
    assert!(response.usage.total_tokens > 0);
}

#[tokio::test]
async fn mock_model_returns_structured_output_when_requested() {
    let model = MockModel::new("mock-dev");
    let response = model
        .complete(&ModelRequest {
            system_prompt: "Be concise.".to_owned(),
            messages: vec![ConversationMessage::user("- body: inspect the queue")],
            tools: Vec::new(),
            response_schema: Some(StructuredOutputSchema {
                name: "queue_summary".to_owned(),
                schema: json!({
                    "type": "object",
                    "properties": {
                        "message": { "type": "string" },
                        "accepted": { "type": "boolean" }
                    },
                    "required": ["message", "accepted"]
                }),
            }),
            controls: Default::default(),
        })
        .await
        .expect("mock model should return structured output");

    assert_eq!(
        response.structured_output,
        Some(json!({
            "message": "Acknowledged: inspect the queue",
            "accepted": true
        }))
    );
    assert_eq!(
        serde_json::from_str::<serde_json::Value>(&response.text())
            .expect("assistant message should be JSON"),
        json!({
            "message": "Acknowledged: inspect the queue",
            "accepted": true
        })
    );
}

#[test]
fn typed_model_error_helpers_preserve_branchable_variants() {
    let cases = [
        (
            ModelError::rate_limited("openai", "gpt-5.5", "HTTP 429"),
            ErrorKind::RateLimited,
            "openai",
            "gpt-5.5",
        ),
        (
            ModelError::transient("anthropic", "claude", "HTTP 500"),
            ErrorKind::Transient,
            "anthropic",
            "claude",
        ),
        (
            ModelError::interrupted("deepseek", "deepseek-reasoner", "stream closed"),
            ErrorKind::Interrupted,
            "deepseek",
            "deepseek-reasoner",
        ),
        (
            ModelError::context_limit_exceeded("google", "gemini", "too large"),
            ErrorKind::ContextLimitExceeded,
            "google",
            "gemini",
        ),
    ];
    for (error, kind, provider, model_id) in cases {
        assert_eq!(error.core().kind, kind);
        assert_eq!(error.core().provider, provider);
        assert_eq!(error.core().model_id, model_id);
    }
}
