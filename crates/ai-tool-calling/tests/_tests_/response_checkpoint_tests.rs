//! Response checkpoints set operation ids and reject invalid finish/call pairs.

use std::sync::{Arc, Mutex};

use ai_interface::{
    ConversationMessage, ConversationPart, ErrorKind, FinishReason, ModelResponse, ToolDefinition,
    ToolInvocation, ToolMock, test_support::model_response_stream,
};
use ai_tool_calling::{
    Agent, AgentEvent, Error, ModelResponseCheckpoint, NoopTurnCheckpoint, Result,
};
use serde_json::json;
use unimock::{MockFn, Unimock, matching};

use super::support::{call, collect, model, request, response, runtime};

#[unimock::unimock(api=ResponseBehaviorMock)]
trait ResponseBehavior: Send {
    fn checkpoint(&mut self, response: &mut ModelResponse) -> Result<()>;
}

struct Checkpoint {
    behavior: Unimock,
}

impl ModelResponseCheckpoint for Checkpoint {
    fn checkpoint_response(&mut self, response: &mut ModelResponse) -> Result<()> {
        self.behavior.checkpoint(response)
    }
}

#[tokio::test]
async fn response_checkpoint_finish_call_mismatches_do_not_advance_the_turn() {
    for (case, finish_reason, keep_calls) in [
        (
            "tool-call finish without calls",
            FinishReason::ToolCalls,
            false,
        ),
        ("stop finish with calls", FinishReason::Stop, true),
    ] {
        let mut checkpoint = Checkpoint {
            behavior: Unimock::new(
                ResponseBehaviorMock::checkpoint
                    .each_call(matching!(_))
                    .answers_arc(Arc::new(move |_, response: &mut ModelResponse| {
                        response.finish_reason = finish_reason.clone();
                        response.parts.retain(|part| {
                            keep_calls || !matches!(part, ConversationPart::ToolCall { .. })
                        });
                        Ok(())
                    })),
            ),
        };
        let (model, _) = model(vec![Ok(model_response_stream(response(
            "working",
            vec![call("provider-call", "save")],
            FinishReason::ToolCalls,
        )))]);
        let agent = runtime(model, Vec::new());
        let mut conversation = vec![ConversationMessage::user("start")];
        let original_conversation = conversation.clone();
        let result = collect(
            agent
                .run_with_checkpoints(
                    &mut conversation,
                    request(Some(1)),
                    &mut NoopTurnCheckpoint,
                    &mut checkpoint,
                )
                .as_mut(),
        )
        .await;

        assert!(
            matches!(&result, Err(Error::Model(error)) if error.core().kind == ErrorKind::InvalidOutput),
            "{case}: expected InvalidOutput, got {result:?}"
        );
        assert_eq!(conversation, original_conversation, "{case}");
    }
}

#[tokio::test]
async fn response_checkpoint_operation_ids_reach_events_conversation_and_execution() {
    let captured = Arc::new(Mutex::new(Vec::new()));
    let tool = Arc::new(Unimock::new((
        ToolMock::definitions
            .each_call(matching!())
            .returns(vec![ToolDefinition {
                name: "save".to_owned(),
                description: "Save a value.".to_owned(),
                input_schema: json!({"type":"object"}),
                activity_verb: None,
            }]),
        ToolMock::call_with_invocation
            .some_call(matching!(_))
            .answers_arc({
                let captured = captured.clone();
                Arc::new(move |_, invocation: ToolInvocation| {
                    captured.lock().unwrap().push(invocation.operation_id);
                    Ok(json!(true))
                })
            }),
    )));
    let checkpoint_responses = Arc::new(Mutex::new(Vec::new()));
    let mut checkpoint = Checkpoint {
        behavior: Unimock::new(
            ResponseBehaviorMock::checkpoint
                .each_call(matching!(_))
                .answers_arc({
                    let checkpoint_responses = checkpoint_responses.clone();
                    Arc::new(move |_, response: &mut ModelResponse| {
                        checkpoint_responses
                            .lock()
                            .unwrap()
                            .push(response.finish_reason.clone());
                        for part in &mut response.parts {
                            if let ConversationPart::ToolCall { operation_id, .. } = part {
                                *operation_id = Some("operation-from-checkpoint".to_owned());
                            }
                        }
                        Ok(())
                    })
                }),
        ),
    };
    let (model, _) = model(vec![
        Ok(model_response_stream(response(
            "working",
            vec![call("provider-call", "save")],
            FinishReason::ToolCalls,
        ))),
        Ok(model_response_stream(response(
            "done",
            vec![],
            FinishReason::Stop,
        ))),
    ]);
    let agent = runtime(model, vec![tool]);
    let mut conversation = vec![ConversationMessage::user("start")];
    let events = collect(
        agent
            .run_with_checkpoints(
                &mut conversation,
                request(None),
                &mut NoopTurnCheckpoint,
                &mut checkpoint,
            )
            .as_mut(),
    )
    .await
    .unwrap();
    assert_eq!(
        *checkpoint_responses.lock().unwrap(),
        [FinishReason::ToolCalls, FinishReason::Stop]
    );
    assert_eq!(*captured.lock().unwrap(), ["operation-from-checkpoint"]);
    assert_eq!(
        conversation[1].tool_calls()[0].operation_id.as_deref(),
        Some("operation-from-checkpoint")
    );
    let first = events
        .iter()
        .find_map(|event| match event {
            AgentEvent::Response(response) => Some(response),
            _ => None,
        })
        .unwrap();
    assert_eq!(
        first.tool_calls()[0].operation_id.as_deref(),
        Some("operation-from-checkpoint")
    );
    let record = events
        .iter()
        .find_map(|event| match event {
            AgentEvent::ToolFinished {
                result: Ok(record), ..
            } => Some(record),
            _ => None,
        })
        .unwrap();
    assert_eq!(record.operation_id, "operation-from-checkpoint");
}
