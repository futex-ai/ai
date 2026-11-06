//! Controllable futures for parallel tools without clocks or sleeps.

use std::{
    collections::BTreeMap,
    future::Future,
    pin::Pin,
    sync::{
        Arc, Mutex,
        atomic::{AtomicUsize, Ordering},
    },
    task::{Context, Poll, Waker},
};

use ai_interface::{DynTool, Tool, ToolDefinition, ToolInvocation, ToolResult};
use ai_tool_calling::{AgentEvent, AgentEventStream};
use async_trait::async_trait;
use serde_json::{Value, json};
use unimock::{MockFn, Unimock, matching};

type ToolFuture = Pin<Box<dyn Future<Output = ToolResult<Value>> + Send>>;

#[unimock::unimock(api = ToolBehaviorMock)]
trait ToolBehavior: Send + Sync {
    fn invoke(&self, invocation: ToolInvocation) -> ToolFuture;
}

struct ControlledTool {
    behavior: Unimock,
}

#[async_trait]
impl Tool for ControlledTool {
    fn definitions(&self) -> Vec<ToolDefinition> {
        vec![ToolDefinition {
            name: "controlled".to_owned(),
            description: "Controlled tool.".to_owned(),
            input_schema: json!({"type":"object"}),
            activity_verb: Some("Running".to_owned()),
        }]
    }

    async fn call(&self, tool_name: &str, input: Value) -> ToolResult<Value> {
        self.call_with_invocation(ToolInvocation {
            tool_name: tool_name.to_owned(),
            input,
            operation_id: String::new(),
        })
        .await
    }

    async fn call_with_invocation(&self, invocation: ToolInvocation) -> ToolResult<Value> {
        self.behavior.invoke(invocation).await
    }

    fn group_for_tool(&self, _: &str) -> Option<&'static str> {
        Some("control")
    }
}

#[derive(Default)]
struct GateState {
    result: Option<ToolResult<Value>>,
    waker: Option<Waker>,
}

#[derive(Clone, Default)]
pub(super) struct Gate {
    state: Arc<Mutex<GateState>>,
    cancelled: Arc<AtomicUsize>,
}

impl Gate {
    pub(super) fn complete(&self, value: Value) {
        self.resolve(Ok(value));
    }

    pub(super) fn resolve(&self, result: ToolResult<Value>) {
        let waker = {
            let mut state = self.state.lock().unwrap();
            state.result = Some(result);
            state.waker.take()
        };
        if let Some(waker) = waker {
            waker.wake();
        }
    }

    pub(super) fn cancelled(&self) -> usize {
        self.cancelled.load(Ordering::SeqCst)
    }

    fn future(&self) -> ToolFuture {
        Box::pin(GateFuture {
            gate: self.clone(),
            done: false,
        })
    }
}

struct GateFuture {
    gate: Gate,
    done: bool,
}

impl Future for GateFuture {
    type Output = ToolResult<Value>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let result = {
            let mut state = self.gate.state.lock().unwrap();
            state.waker = Some(cx.waker().clone());
            state.result.take()
        };
        match result {
            Some(result) => {
                self.done = true;
                Poll::Ready(result)
            }
            None => Poll::Pending,
        }
    }
}

impl Drop for GateFuture {
    fn drop(&mut self) {
        if !self.done {
            self.gate.cancelled.fetch_add(1, Ordering::SeqCst);
        }
    }
}

pub(super) struct ControlledCalls {
    pub(super) tool: DynTool,
    pub(super) gates: Vec<Gate>,
    pub(super) started: Arc<Mutex<Vec<String>>>,
}

impl ControlledCalls {
    pub(super) fn unstarted() -> Self {
        Self {
            tool: Arc::new(ControlledTool {
                behavior: Unimock::new(()),
            }),
            gates: Vec::new(),
            started: Arc::new(Mutex::new(Vec::new())),
        }
    }

    pub(super) fn new(count: usize) -> Self {
        let gates = (0..count).map(|_| Gate::default()).collect::<Vec<_>>();
        let by_id = gates
            .iter()
            .enumerate()
            .map(|(i, gate)| (format!("c{i}"), gate.clone()))
            .collect::<BTreeMap<_, _>>();
        let started = Arc::new(Mutex::new(Vec::new()));
        let behavior = Unimock::new(
            ToolBehaviorMock::invoke
                .each_call(matching!(_))
                .answers_arc({
                    let started = started.clone();
                    Arc::new(move |_, invocation: ToolInvocation| {
                        started
                            .lock()
                            .unwrap()
                            .push(invocation.operation_id.clone());
                        by_id
                            .get(&invocation.operation_id)
                            .expect("known gate")
                            .future()
                    })
                }),
        );
        Self {
            tool: Arc::new(ControlledTool { behavior }),
            gates,
            started,
        }
    }
}

pub(super) fn assert_pending(stream: &mut dyn AgentEventStream) {
    let mut next = stream.next();
    let mut context = Context::from_waker(Waker::noop());
    assert!(
        next.as_mut().poll(&mut context).is_pending(),
        "the blocked turn must stay pending"
    );
}

pub(super) async fn next_tool_event(stream: &mut dyn AgentEventStream) -> AgentEvent {
    loop {
        let event = stream
            .next()
            .await
            .expect("tool event result")
            .expect("tool event");
        if matches!(
            event,
            AgentEvent::ToolStarted { .. } | AgentEvent::ToolFinished { .. }
        ) {
            return event;
        }
    }
}

pub(super) fn ready_tool_event(stream: &mut dyn AgentEventStream) -> AgentEvent {
    let mut next = Box::pin(next_tool_event(stream));
    let mut context = Context::from_waker(Waker::noop());
    let result = next.as_mut().poll(&mut context);
    assert!(
        result.is_ready(),
        "a runnable tool event must be ready without a blocked earlier call"
    );
    let Poll::Ready(event) = result else {
        unreachable!()
    };
    event
}
