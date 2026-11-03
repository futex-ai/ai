//! Terminal outcomes, step caps, and final-answer schema propagation.

use std::sync::Arc;

use ai_interface::{
    ConversationMessage, ErrorKind, FinishReason, ModelError, StructuredOutputSchema,
    ToolDefinition, ToolMock, test_support::model_response_stream,
};
use ai_tool_calling::{Agent, AgentEvent, Error, RunOutcome};
use serde_json::json;
use unimock::{MockFn, Unimock, matching};

use super::support::{call, collect, model, outcome, request, response, runtime};

#[tokio::test]
async fn sends_final_schema_on_every_step_and_returns_structured_output() {
    let tool = Arc::new(Unimock::new((
        ToolMock::definitions
            .each_call(matching!())
            .returns(vec![ToolDefinition {
                name: "lookup".to_owned(),
                description: "Look up data.".to_owned(),
                input_schema: json!({"type":"object"}),
                activity_verb: None,
            }]),
        ToolMock::call
            .some_call(matching!("lookup", _))
            .returns(Ok(json!({"found":true}))),
    )));
    let mut final_response = response("{\"ok\":true}", vec![], FinishReason::Stop);
    final_response.structured_output = Some(json!({"ok":true}));
    let (model, requests) = model(vec![
        Ok(model_response_stream(response(
            "checking",
            vec![call("lookup-1", "lookup")],
            FinishReason::ToolCalls,
        ))),
        Ok(model_response_stream(final_response)),
    ]);
    let agent = runtime(model, vec![tool]);
    let mut request = request(None);
    request.controls.generation.max_output_tokens = Some(99);
    request.final_answer_schema = Some(StructuredOutputSchema {
        name: "answer".to_owned(),
        schema: json!({"type":"object","properties":{"ok":{"type":"boolean"}},"required":["ok"]}),
    });
    let mut conversation = vec![ConversationMessage::user("start")];
    let events = collect(agent.run(&mut conversation, request.clone()).as_mut())
        .await
        .unwrap();
    assert_eq!(
        outcome(&events),
        &RunOutcome::Completed {
            assistant_message: "{\"ok\":true}".to_owned(),
            structured_output: Some(json!({"ok":true})),
            steps_taken: 2,
        }
    );
    let requests = requests.lock().unwrap();
    assert_eq!(requests.len(), 2);
    for sent in requests.iter() {
        assert_eq!(sent.response_schema, request.final_answer_schema);
        assert_eq!(sent.controls, request.controls);
        assert_eq!(sent.system_prompt, "Use the tools.");
    }
}

#[tokio::test]
async fn zero_step_limit_caps_without_a_model_call_or_append() {
    let agent = runtime(Arc::new(Unimock::new(())), vec![]);
    let mut conversation = vec![ConversationMessage::user("unchanged")];
    let events = collect(agent.run(&mut conversation, request(Some(0))).as_mut())
        .await
        .unwrap();
    assert_eq!(events.len(), 1);
    assert_eq!(
        outcome(&events),
        &RunOutcome::Capped {
            assistant_message: String::new(),
            steps_taken: 0,
            max_steps: 0
        }
    );
    assert_eq!(conversation, vec![ConversationMessage::user("unchanged")]);
}

#[tokio::test]
async fn filtered_and_other_finishes_append_nothing_and_end_with_the_provider_error() {
    for (reason, message) in [
        (FinishReason::Filtered, "filtered"),
        (
            FinishReason::Other("provider-special-finish".to_owned()),
            "provider-special-finish",
        ),
    ] {
        let (model, _) = model(vec![Ok(model_response_stream(response(
            "not appended",
            vec![],
            reason,
        )))]);
        let agent = runtime(model, vec![]);
        let mut conversation = vec![ConversationMessage::user("original")];
        let error = {
            let mut stream = agent.run(&mut conversation, request(None));
            let error = collect(stream.as_mut()).await.unwrap_err();
            assert!(stream.next().await.unwrap().is_none());
            error
        };
        let Error::Model(error) = error else {
            panic!("expected provider error")
        };
        assert_eq!(error.core().kind, ErrorKind::InvalidOutput);
        assert!(error.core().message.contains(message));
        assert_eq!(conversation, vec![ConversationMessage::user("original")]);
    }
}

#[tokio::test]
async fn invalid_schema_output_propagates_the_model_error_without_an_append() {
    let (model, _) = model(vec![Err(ModelError::invalid_output(
        "test",
        "test-model",
        "schema mismatch",
    ))]);
    let agent = runtime(model, vec![]);
    let mut conversation = vec![ConversationMessage::user("original")];
    let mut request = request(None);
    request.final_answer_schema = Some(StructuredOutputSchema {
        name: "answer".to_owned(),
        schema: json!({"type":"object"}),
    });
    let error = collect(agent.run(&mut conversation, request).as_mut())
        .await
        .unwrap_err();
    let Error::Model(error) = error else {
        panic!("expected model error")
    };
    assert_eq!(error.core().kind, ErrorKind::InvalidOutput);
    assert_eq!(error.core().message, "schema mismatch");
    assert_eq!(conversation, vec![ConversationMessage::user("original")]);
}

#[tokio::test]
async fn an_empty_stop_appends_no_assistant_message() {
    let (model, _) = model(vec![Ok(model_response_stream(response(
        "",
        vec![],
        FinishReason::Stop,
    )))]);
    let agent = runtime(model, vec![]);
    let mut conversation = vec![ConversationMessage::user("original")];
    let events = collect(agent.run(&mut conversation, request(None)).as_mut())
        .await
        .unwrap();
    let messages = events
        .iter()
        .find_map(|event| match event {
            AgentEvent::StepFinished { messages } => Some(messages),
            _ => None,
        })
        .unwrap();
    assert!(messages.is_empty());
    assert_eq!(conversation, vec![ConversationMessage::user("original")]);
    assert_eq!(
        outcome(&events),
        &RunOutcome::Completed {
            assistant_message: String::new(),
            steps_taken: 1,
            structured_output: None
        }
    );
}
