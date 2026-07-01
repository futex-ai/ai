use std::sync::Arc;

use ai_interface::{
    ConversationMessage, ConversationRole, FinishReason, Model, ModelMock, ModelResponse,
    ModelUsage,
};
use unimock::{MockFn, Unimock, matching};

use crate::Turn;

use super::support::{runtime, user_message};

#[tokio::test]
async fn conversation_mutation_and_send_flow_work() {
    let model: Arc<dyn Model> = Arc::new(Unimock::new(
        ModelMock::complete
            .next_call(matching!(_))
            .returns(Ok(ModelResponse {
                provider: "mock".to_owned(),
                model_id: "mock-model".to_owned(),
                catalog_model_id: None,
                thinking_level: None,
                assistant_message: "done".to_owned(),
                tool_calls: Vec::new(),
                finish_reason: FinishReason::Stop,
                structured_output: None,
                provider_context: Vec::new(),
                usage: ModelUsage::default(),
            })),
    ));
    let runtime = runtime(model, Vec::new()).expect("runtime should build");

    runtime.push_user_message("seed user");
    runtime.push_assistant_message("seed assistant");
    assert_eq!(runtime.conversation().len(), 2);

    runtime.replace_conversation(vec![ConversationMessage {
        role: ConversationRole::User,
        content: "replaced".to_owned(),
        content_parts: Vec::new(),
        name: None,
        tool_call_id: None,
        tool_calls: Vec::new(),
        provider_context: Vec::new(),
    }]);
    assert_eq!(runtime.conversation().len(), 1);

    let mut turn = runtime.send(user_message("hello"), Some(4));
    assert_eq!(runtime.conversation().len(), 2);
    let outcome = turn.step().await.expect("step should succeed");
    assert!(matches!(
        outcome,
        crate::StepOutcome::Completed {
            assistant_message,
            steps_taken: 1,
        } if assistant_message == "done"
    ));

    runtime.clear_conversation();
    assert!(runtime.conversation().is_empty());
}

#[tokio::test]
async fn resume_starts_turn_without_appending_user_message() {
    let model: Arc<dyn Model> = Arc::new(Unimock::new(
        ModelMock::complete
            .next_call(matching!(_))
            .answers(&|_, request| {
                assert_eq!(request.messages.len(), 1);
                assert_eq!(request.messages[0].content, "already retained");
                Ok(ModelResponse {
                    provider: "mock".to_owned(),
                    model_id: "mock-model".to_owned(),
                    catalog_model_id: None,
                    thinking_level: None,
                    assistant_message: "done".to_owned(),
                    tool_calls: Vec::new(),
                    finish_reason: FinishReason::Stop,
                    structured_output: None,
                    provider_context: Vec::new(),
                    usage: ModelUsage::default(),
                })
            }),
    ));
    let runtime = runtime(model, Vec::new()).expect("runtime should build");
    runtime.replace_conversation(vec![user_message("already retained")]);

    let mut turn = runtime.resume(Some(4));
    let outcome = turn.step().await.expect("step should succeed");

    assert!(matches!(
        outcome,
        crate::StepOutcome::Completed {
            assistant_message,
            steps_taken: 1,
        } if assistant_message == "done"
    ));
}
