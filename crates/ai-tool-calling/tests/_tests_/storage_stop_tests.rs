//! A completed tool output survives waiting storage and internal failures.

use std::{
    future::Future,
    pin::Pin,
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    },
    task::{Context, Poll},
};

use ai_interface::{
    ConversationMessage, ConversationPart, FinishReason, ToolActivityPhase, ToolCallLogResult,
    test_support::model_response_stream,
};
use ai_tool_calling::{
    Agent, AgentEvent, Error, NoopModelResponseCheckpoint, ToolCallingRuntime, ToolOutputPolicy,
    ToolOutputStore, ToolOutputStoreError, ToolOutputStoreReadRequest, ToolOutputStoreResult,
    ToolOutputStoreWindow, ToolOutputWriteRequest, ToolOutputWriteResult,
};
use async_trait::async_trait;
use serde_json::{Value, json};
use unimock::{MockFn, Unimock, matching};

use super::{
    checkpoint_tests::counting_checkpoint,
    controlled::{ControlledCalls, assert_pending, ready_tool_event},
    support::{call, collect, model, observe_tools, request, response},
};

type WriteFuture =
    Pin<Box<dyn Future<Output = ToolOutputStoreResult<ToolOutputWriteResult>> + Send>>;

#[unimock::unimock(api=StoreBehaviorMock)]
trait StoreBehavior: Send + Sync {
    fn write(&self, request: ToolOutputWriteRequest) -> WriteFuture;
}

struct Store {
    behavior: Unimock,
}

#[async_trait]
impl ToolOutputStore for Store {
    fn has_outputs(&self) -> bool {
        false
    }

    async fn write(
        &self,
        request: ToolOutputWriteRequest,
    ) -> ToolOutputStoreResult<ToolOutputWriteResult> {
        StoreBehavior::write(&self.behavior, request).await
    }

    async fn read(
        &self,
        _: ToolOutputStoreReadRequest,
    ) -> ToolOutputStoreResult<ToolOutputStoreWindow> {
        unreachable!("this fixture offers no stored output")
    }
}

struct PendingWrite {
    cancelled: Arc<AtomicUsize>,
}

impl Future for PendingWrite {
    type Output = ToolOutputStoreResult<ToolOutputWriteResult>;
    fn poll(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Self::Output> {
        Poll::Pending
    }
}

impl Drop for PendingWrite {
    fn drop(&mut self) {
        self.cancelled.fetch_add(1, Ordering::SeqCst);
    }
}

#[tokio::test]
async fn dropping_during_a_store_write_keeps_the_already_completed_tool_result() {
    let cancelled = Arc::new(AtomicUsize::new(0));
    let behavior = Unimock::new(
        StoreBehaviorMock::write
            .some_call(matching!(_))
            .answers_arc({
                let cancelled = cancelled.clone();
                Arc::new(move |_, _: ToolOutputWriteRequest| -> WriteFuture {
                    Box::pin(PendingWrite {
                        cancelled: cancelled.clone(),
                    })
                })
            }),
    );
    let controls = ControlledCalls::new(1);
    controls.gates[0].complete(json!("completed-before-storage"));
    let observations = observe_tools();
    let (mut checkpoint, checkpoint_count) = counting_checkpoint();
    let mut response_checkpoint = NoopModelResponseCheckpoint;
    let (model, _) = model(vec![Ok(model_response_stream(response(
        "working",
        vec![call("c0", "controlled")],
        FinishReason::ToolCalls,
    )))]);
    let agent = ToolCallingRuntime::new(
        model,
        observations.logger,
        vec![controls.tool.clone()],
        Arc::new(Store { behavior }),
        ToolOutputPolicy::new(8, 8, 128, 256).unwrap(),
    )
    .unwrap();
    let mut conversation = vec![ConversationMessage::user("start")];
    {
        let mut stream = agent.run_with_checkpoints(
            &mut conversation,
            request(None),
            &mut checkpoint,
            &mut response_checkpoint,
        );
        assert!(
            matches!(ready_tool_event(stream.as_mut()),AgentEvent::ToolStarted {call_id,..} if call_id=="c0")
        );
        assert_pending(stream.as_mut());
    }
    assert_eq!(conversation.len(), 3);
    let ConversationPart::ToolResult {
        call_id, content, ..
    } = &conversation[2].parts[0]
    else {
        panic!("completed output missing")
    };
    let content: Value = serde_json::from_str(content).unwrap();
    assert_eq!(call_id, "c0");
    assert_eq!(content["type"], "tool_output_window");
    assert_eq!(content["content"], "\"complet");
    assert_eq!(content["remainder_unavailable"], "store_unavailable");
    assert!(content.get("output_id").is_none());
    assert_eq!(controls.gates[0].cancelled(), 0);
    assert_eq!(cancelled.load(Ordering::SeqCst), 1);
    assert_eq!(
        checkpoint_count.load(Ordering::SeqCst),
        4,
        "a completed raw output needs its checkpoint when a pending write is dropped"
    );
    let calls = observations.calls.lock().unwrap();
    assert_eq!(
        calls.len(),
        1,
        "a completed result needs an observer callback even during a cancelled write"
    );
    assert!(matches!(calls[0].result, ToolCallLogResult::Success { .. }));
    assert_eq!(
        observations
            .activities
            .lock()
            .unwrap()
            .iter()
            .map(|entry| entry.phase)
            .collect::<Vec<_>>(),
        [ToolActivityPhase::Started, ToolActivityPhase::Completed]
    );
}

#[tokio::test]
async fn an_internal_error_keeps_all_received_outputs_before_returning_the_error() {
    let store = Store {
        behavior: Unimock::new(StoreBehaviorMock::write.some_call(matching!(_)).answers(
            &|_, _| -> WriteFuture {
                Box::pin(async { Err(ToolOutputStoreError::InvalidLength { length: 0 }) })
            },
        )),
    };
    let controls = ControlledCalls::new(3);
    controls.gates[1].complete(json!(1));
    controls.gates[2].complete(json!("completed-before-failure"));
    let observations = observe_tools();
    let (mut checkpoint, checkpoint_count) = counting_checkpoint();
    let mut response_checkpoint = NoopModelResponseCheckpoint;
    let (model, _) = model(vec![Ok(model_response_stream(response(
        "working",
        (0..3)
            .map(|i| call(&format!("c{i}"), "controlled"))
            .collect(),
        FinishReason::ToolCalls,
    )))]);
    let agent = ToolCallingRuntime::new(
        model,
        observations.logger,
        vec![controls.tool.clone()],
        Arc::new(store),
        ToolOutputPolicy::new(8, 8, 128, 256).unwrap(),
    )
    .unwrap();
    let mut conversation = vec![ConversationMessage::user("start")];
    {
        let mut stream = agent.run_with_checkpoints(
            &mut conversation,
            request(None),
            &mut checkpoint,
            &mut response_checkpoint,
        );
        assert!(
            matches!(ready_tool_event(stream.as_mut()),AgentEvent::ToolStarted {call_id,..} if call_id=="c0")
        );
        assert!(
            matches!(ready_tool_event(stream.as_mut()),AgentEvent::ToolStarted {call_id,..} if call_id=="c1")
        );
        assert!(
            matches!(ready_tool_event(stream.as_mut()),AgentEvent::ToolFinished {call_id,..} if call_id=="c1")
        );
        assert!(
            matches!(ready_tool_event(stream.as_mut()),AgentEvent::ToolStarted {call_id,..} if call_id=="c2")
        );
        assert_pending(stream.as_mut());
        controls.gates[0].complete(json!(0));
        let result = collect(stream.as_mut()).await;
        assert!(
            matches!(result, Err(Error::UnexpectedOutputStoreWrite { .. })),
            "expected original internal failure: {result:?}"
        );
        assert!(stream.next().await.unwrap().is_none());
    }
    let ids = conversation
        .iter()
        .flat_map(|message| &message.parts)
        .filter_map(|part| match part {
            ConversationPart::ToolResult { call_id, .. } => Some(call_id.as_str()),
            _ => None,
        })
        .collect::<Vec<_>>();
    assert_eq!(ids, ["c0", "c1", "c2"]);
    let output: Value = serde_json::from_str(&conversation[4].text()).unwrap();
    assert_eq!(output["remainder_unavailable"], "store_unavailable");
    assert_eq!(output["content"], "\"complet");
    assert_eq!(controls.gates[0].cancelled(), 0);
    assert_eq!(
        observations.calls.lock().unwrap().len(),
        3,
        "each completed raw result needs an observer callback"
    );
    assert_eq!(
        observations
            .activities
            .lock()
            .unwrap()
            .iter()
            .filter(|entry| entry.phase == ToolActivityPhase::Completed)
            .count(),
        3
    );
    assert_eq!(
        checkpoint_count.load(Ordering::SeqCst),
        6,
        "checkpoints must run after internally failed output management too"
    );
}
