//! Parallel execution, ordered appends, and stopped-turn preservation.

use std::num::NonZeroUsize;

use ai_interface::{
    ConversationMessage, ConversationPart, FinishReason, test_support::model_response_stream,
};
use ai_tool_calling::{Agent, AgentEvent};
use serde_json::json;

use super::{
    controlled::{ControlledCalls, assert_pending, ready_tool_event},
    support::{call, collect, model, request, response, runtime},
};

fn result_ids(messages: &[ConversationMessage]) -> Vec<&str> {
    messages
        .iter()
        .flat_map(|message| &message.parts)
        .filter_map(|part| match part {
            ConversationPart::ToolResult { call_id, .. } => Some(call_id.as_str()),
            _ => None,
        })
        .collect()
}

#[tokio::test]
async fn limits_parallel_calls_and_appends_out_of_order_completions_in_call_order() {
    for limit in [None, NonZeroUsize::new(2)] {
        let count = 5;
        let controls = ControlledCalls::new(count);
        let calls = (0..count)
            .map(|index| call(&format!("c{index}"), "controlled"))
            .collect();
        let (model, requests) = model(vec![
            Ok(model_response_stream(response(
                "working",
                calls,
                FinishReason::ToolCalls,
            ))),
            Ok(model_response_stream(response(
                "done",
                vec![],
                FinishReason::Stop,
            ))),
        ]);
        let mut agent = runtime(model, vec![controls.tool.clone()]);
        if let Some(limit) = limit {
            agent = agent.with_tool_call_limit(limit);
        }
        let active = limit.map(NonZeroUsize::get).unwrap_or(4);
        let mut conversation = vec![ConversationMessage::user("start")];
        let events = {
            let mut stream = agent.run(&mut conversation, request(None));
            for index in 0..active {
                assert!(
                    matches!(ready_tool_event(stream.as_mut()), AgentEvent::ToolStarted {call_id, ..} if call_id == format!("c{index}"))
                );
            }
            assert_pending(stream.as_mut());
            assert_eq!(
                controls.started.lock().unwrap().as_slice(),
                &(0..active)
                    .map(|index| format!("c{index}"))
                    .collect::<Vec<_>>()
            );
            controls.gates[1].complete(json!({"value":1}));
            assert!(
                matches!(ready_tool_event(stream.as_mut()), AgentEvent::ToolFinished {call_id, result: Ok(_)} if call_id == "c1")
            );
            assert!(
                matches!(ready_tool_event(stream.as_mut()), AgentEvent::ToolStarted {call_id, ..} if call_id == format!("c{active}"))
            );
            for (index, gate) in controls.gates.iter().enumerate() {
                if index != 1 {
                    gate.complete(json!({"value":index}));
                }
            }
            collect(stream.as_mut()).await.unwrap()
        };
        let messages = events
            .iter()
            .find_map(|event| match event {
                AgentEvent::StepFinished { messages } => Some(messages),
                _ => None,
            })
            .unwrap();
        assert_eq!(messages[0].text(), "working");
        assert_eq!(result_ids(messages), ["c0", "c1", "c2", "c3", "c4"]);
        assert_eq!(result_ids(&conversation), ["c0", "c1", "c2", "c3", "c4"]);
        assert_eq!(
            requests.lock().unwrap()[1].messages,
            conversation[..conversation.len() - 1]
        );
    }
}

#[tokio::test]
async fn dropping_stream_keeps_waiting_results_and_cancels_unfinished_calls() {
    let controls = ControlledCalls::new(3);
    let (model, _) = model(vec![Ok(model_response_stream(response(
        "working",
        (0..3)
            .map(|index| call(&format!("c{index}"), "controlled"))
            .collect(),
        FinishReason::ToolCalls,
    )))]);
    let agent = runtime(model, vec![controls.tool.clone()])
        .with_tool_call_limit(NonZeroUsize::new(2).unwrap());
    let mut conversation = vec![ConversationMessage::user("start")];
    {
        let mut stream = agent.run(&mut conversation, request(None));
        for _ in 0..2 {
            assert!(matches!(
                ready_tool_event(stream.as_mut()),
                AgentEvent::ToolStarted { .. }
            ));
        }
        assert_pending(stream.as_mut());
        controls.gates[1].complete(json!({"saved":true}));
        assert!(
            matches!(ready_tool_event(stream.as_mut()), AgentEvent::ToolFinished {call_id, result: Ok(_)} if call_id == "c1")
        );
    }
    assert_eq!(result_ids(&conversation), ["c1"]);
    assert_eq!(conversation[1].text(), "working");
    assert_eq!(controls.gates[0].cancelled(), 1);
    assert_eq!(controls.gates[2].cancelled(), 0);
    assert_eq!(controls.started.lock().unwrap().as_slice(), ["c0", "c1"]);
}

#[tokio::test]
async fn assistant_is_appended_before_any_tool_starts() {
    let controls = ControlledCalls::unstarted();
    let (model, _) = model(vec![Ok(model_response_stream(response(
        "working",
        vec![call("c0", "controlled")],
        FinishReason::ToolCalls,
    )))]);
    let agent = runtime(model, vec![controls.tool.clone()]);
    let mut conversation = vec![ConversationMessage::user("start")];
    {
        let mut stream = agent.run(&mut conversation, request(None));
        while let Some(event) = stream.next().await.unwrap() {
            if matches!(event, AgentEvent::Response(_)) {
                break;
            }
        }
    }
    assert_eq!(conversation.len(), 2);
    assert_eq!(conversation[1].tool_calls()[0].id, "c0");
    assert_eq!(controls.started.lock().unwrap().len(), 0);
}
