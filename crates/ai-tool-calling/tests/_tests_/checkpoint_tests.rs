//! A checkpoint stop drains running calls without starting new work.

use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};

use ai_interface::{
    ConversationMessage, ConversationPart, FinishReason, ModelError,
    test_support::model_response_stream,
};
use ai_tool_calling::{
    Agent, AgentEvent, Error, NoopModelResponseCheckpoint, Result, TurnCheckpoint,
};
use serde_json::json;
use unimock::{MockFn, Unimock, matching};

use super::{
    controlled::{ControlledCalls, assert_pending, ready_tool_event},
    support::{call, collect, model, request, response, runtime},
};

#[unimock::unimock(api = CheckpointBehaviorMock)]
trait CheckpointBehavior: Send {
    fn check(&mut self) -> Result<()>;
}

pub(super) struct Checkpoint {
    behavior: Unimock,
}

pub(super) fn counting_checkpoint() -> (Checkpoint, Arc<AtomicUsize>) {
    let count = Arc::new(AtomicUsize::new(0));
    let checkpoint = Checkpoint {
        behavior: Unimock::new(
            CheckpointBehaviorMock::check
                .each_call(matching!())
                .answers_arc({
                    let count = count.clone();
                    Arc::new(move |_| {
                        count.fetch_add(1, Ordering::SeqCst);
                        Ok(())
                    })
                }),
        ),
    };
    (checkpoint, count)
}

impl TurnCheckpoint for Checkpoint {
    fn check(&mut self) -> Result<()> {
        CheckpointBehavior::check(&mut self.behavior)
    }
}

#[derive(Debug, thiserror::Error)]
#[error("checkpoint stopped")]
struct Stopped;

fn ids(conversation: &[ConversationMessage]) -> Vec<&str> {
    conversation
        .iter()
        .flat_map(|message| &message.parts)
        .filter_map(|part| match part {
            ConversationPart::ToolResult { call_id, .. } => Some(call_id.as_str()),
            _ => None,
        })
        .collect()
}

#[tokio::test]
async fn checkpoint_error_keeps_waiting_results_drains_running_calls_and_starts_no_more() {
    let controls = ControlledCalls::new(3);
    let (model, _) = model(vec![Ok(model_response_stream(response(
        "working",
        (0..3)
            .map(|i| call(&format!("c{i}"), "controlled"))
            .collect(),
        FinishReason::ToolCalls,
    )))]);
    let agent = runtime(model, vec![controls.tool.clone()])
        .with_tool_call_limit(std::num::NonZeroUsize::new(2).unwrap());
    let count = Arc::new(AtomicUsize::new(0));
    let mut checkpoint = Checkpoint {
        behavior: Unimock::new(
            CheckpointBehaviorMock::check
                .each_call(matching!())
                .answers_arc({
                    let count = count.clone();
                    Arc::new(move |_| {
                        if count.fetch_add(1, Ordering::SeqCst) == 3 {
                            Err(Error::checkpoint(Stopped))
                        } else {
                            Ok(())
                        }
                    })
                }),
        ),
    };
    let mut response_checkpoint = NoopModelResponseCheckpoint;
    let mut conversation = vec![ConversationMessage::user("start")];
    {
        let mut stream = agent.run_with_checkpoints(
            &mut conversation,
            request(None),
            &mut checkpoint,
            &mut response_checkpoint,
        );
        for _ in 0..2 {
            assert!(matches!(
                ready_tool_event(stream.as_mut()),
                AgentEvent::ToolStarted { .. }
            ));
        }
        assert_pending(stream.as_mut());
        controls.gates[1].complete(json!({"second":true}));
        assert_pending(stream.as_mut());
        assert_eq!(controls.started.lock().unwrap().as_slice(), ["c0", "c1"]);
        controls.gates[0].complete(json!({"first":true}));
        let result = collect(stream.as_mut()).await;
        assert!(
            matches!(result, Err(Error::Checkpoint { .. })),
            "checkpoint failure must survive a cancelled next future: {result:?}"
        );
        assert!(stream.next().await.unwrap().is_none());
    }
    assert_eq!(count.load(Ordering::SeqCst), 5);
    assert_eq!(ids(&conversation), ["c0", "c1"]);
    assert_eq!(controls.gates[0].cancelled(), 0);
    assert_eq!(controls.gates[2].cancelled(), 0);
    assert_eq!(controls.started.lock().unwrap().as_slice(), ["c0", "c1"]);
}

#[tokio::test]
async fn a_later_model_error_keeps_every_completed_tool_result() {
    let controls = ControlledCalls::new(2);
    controls.gates[0].complete(json!({"first":true}));
    controls.gates[1].complete(json!({"second":true}));
    let (model, _) = model(vec![
        Ok(model_response_stream(response(
            "working",
            vec![call("c0", "controlled"), call("c1", "controlled")],
            FinishReason::ToolCalls,
        ))),
        Err(ModelError::interrupted(
            "test",
            "test-model",
            "stream stopped",
        )),
    ]);
    let agent = runtime(model, vec![controls.tool.clone()]);
    let mut conversation = vec![ConversationMessage::user("start")];
    let error = collect(agent.run(&mut conversation, request(None)).as_mut())
        .await
        .unwrap_err();
    let Error::Model(error) = error else {
        panic!("expected model error")
    };
    assert_eq!(error.core().kind, ai_interface::ErrorKind::Interrupted);
    assert_eq!(ids(&conversation), ["c0", "c1"]);
    assert_eq!(conversation[1].tool_calls().len(), 2);
    assert_eq!(conversation.len(), 4);
}
