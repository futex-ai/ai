//! Runtime tests through the model stream contract.

use std::sync::Arc;

use ai_interface::test_support::model_response_stream;
use ai_interface::{
    ConversationMessage, FinishReason, Model, ModelMock, ModelResponse, ModelUsage,
};
use unimock::{MockFn, Unimock, matching};

use crate::tests::event_support::{collect_events, request, run_outcome};
use crate::tests::support::response_parts;
use crate::{Agent, RunOutcome};

use super::support::{runtime, user_message};

#[tokio::test]
async fn caller_owned_conversation_and_run_flow_work() {
    let model: Arc<dyn Model> = Arc::new(Unimock::new(
        ModelMock::stream
            .next_call(matching!(_))
            .answers(&|_, request| {
                assert_eq!(request.messages.len(), 2);
                assert_eq!(request.messages[0].text(), "replaced");
                assert_eq!(request.messages[1].text(), "hello");
                Ok(model_response_stream(ModelResponse {
                    attempts: Vec::new(),
                    provider: "mock".to_owned(),
                    model_id: "mock-model".to_owned(),
                    catalog_model_id: None,
                    thinking_level: None,
                    parts: response_parts("done".to_owned(), Vec::new(), Vec::new()),
                    finish_reason: FinishReason::Stop,
                    structured_output: None,
                    usage: ModelUsage::default(),
                }))
            }),
    ));
    let runtime = runtime(model, Vec::new()).expect("runtime should build");

    let mut conversation = vec![
        ConversationMessage::user("seed user"),
        ConversationMessage::assistant("seed assistant", Vec::new()),
    ];
    conversation.clear();
    conversation.push(ConversationMessage::user("replaced"));
    conversation.push(user_message("hello"));
    let events = collect_events(runtime.run(&mut conversation, request(Some(4))))
        .await
        .expect("turn should succeed");
    assert!(matches!(
        run_outcome(&events),
        RunOutcome::Completed {
            assistant_message,
            steps_taken: 1,
            ..
        } if assistant_message == "done"
    ));

    assert_eq!(
        conversation,
        vec![
            ConversationMessage::user("replaced"),
            ConversationMessage::user("hello"),
            ConversationMessage::assistant("done", Vec::new()),
        ]
    );
}

#[tokio::test]
async fn run_uses_supplied_history_without_appending_user_message() {
    let model: Arc<dyn Model> = Arc::new(Unimock::new(
        ModelMock::stream
            .next_call(matching!(_))
            .answers(&|_, request| {
                {
                    assert_eq!(request.messages.len(), 1);
                    assert_eq!(request.messages[0].text(), "already retained");
                    Ok(ModelResponse {
                        attempts: Vec::new(),
                        provider: "mock".to_owned(),
                        model_id: "mock-model".to_owned(),
                        catalog_model_id: None,
                        thinking_level: None,
                        parts: response_parts("done".to_owned(), Vec::new(), Vec::new()),
                        finish_reason: FinishReason::Stop,
                        structured_output: None,
                        usage: ModelUsage::default(),
                    })
                }
                .map(model_response_stream)
            }),
    ));
    let runtime = runtime(model, Vec::new()).expect("runtime should build");
    let mut conversation = vec![user_message("already retained")];

    let events = collect_events(runtime.run(&mut conversation, request(Some(4))))
        .await
        .expect("turn should succeed");

    assert!(matches!(
        run_outcome(&events),
        RunOutcome::Completed {
            assistant_message,
            steps_taken: 1,
            ..
        } if assistant_message == "done"
    ));
}
