//! Forwarding and shared-fold behavior at the public stream boundary.

use ai_interface::{
    ConversationMessage, ConversationPart, FinishReason, ModelError, ModelEvent, ModelUsage,
    ProviderId, ProviderState, SafeErrorSource, test_support::model_event_stream,
};
use ai_tool_calling::{Agent, AgentEvent, RunOutcome};
use serde_json::json;

use super::support::{collect, model, model_values, outcome, request, runtime};

#[derive(Debug, thiserror::Error)]
#[error("private failed-attempt cause")]
struct AttemptCause;

#[tokio::test]
async fn forwards_original_events_and_folds_only_the_successful_attempt() {
    let state = ProviderState {
        provider: ProviderId::new("test"),
        model: "test-model".to_owned(),
        version: 1,
        payload: json!({"signature": "private-replay"})
            .as_object()
            .unwrap()
            .clone(),
    };
    let events = vec![
        ModelEvent::TextDelta {
            part_index: 0,
            delta: "discarded".to_owned(),
            provider_state: None,
        },
        ModelEvent::Fallback {
            error: ModelError::internal("failed", "first", SafeErrorSource::redacted(AttemptCause)),
            emitted_events: true,
        },
        ModelEvent::ReasoningDelta {
            part_index: 0,
            delta: "private reasoning".to_owned(),
            provider_state: Some(state.clone()),
        },
        ModelEvent::TextDelta {
            part_index: 1,
            delta: "ans".to_owned(),
            provider_state: None,
        },
        ModelEvent::TextDelta {
            part_index: 1,
            delta: "wer".to_owned(),
            provider_state: Some(state.clone()),
        },
        ModelEvent::Usage {
            usage: ModelUsage::default(),
        },
        ModelEvent::Finish {
            finish_reason: FinishReason::Stop,
            provider: "test".to_owned(),
            model_id: "test-model".to_owned(),
            catalog_model_id: None,
            thinking_level: None,
            structured_output: None,
        },
    ];
    let expected_events = events
        .iter()
        .map(|event| serde_json::to_value(event).unwrap())
        .collect::<Vec<_>>();
    let (model, requests) = model(vec![Ok(model_event_stream(events))]);
    let agent = runtime(model, Vec::new());
    let original = ConversationMessage::user("start");
    let mut conversation = vec![original.clone()];
    let events = collect(agent.run(&mut conversation, request(None)).as_mut())
        .await
        .unwrap();
    assert_eq!(model_values(&events), expected_events);
    let fallback = events
        .iter()
        .find_map(|event| match event {
            AgentEvent::Model(ModelEvent::Fallback { error, .. }) => Some(error),
            _ => None,
        })
        .unwrap();
    let ModelError::Internal { source, .. } = fallback else {
        panic!("lost fallback cause")
    };
    assert!(source.raw_cause().is::<AttemptCause>());
    let response = events
        .iter()
        .find_map(|event| match event {
            AgentEvent::Response(response) => Some(response),
            _ => None,
        })
        .unwrap();
    assert_eq!(response.text(), "answer");
    assert_eq!(response.attempts.len(), 1);
    assert_eq!(
        response.parts,
        vec![
            ConversationPart::reasoning("private reasoning").with_provider_state(state.clone()),
            ConversationPart::text("answer").with_provider_state(state),
        ]
    );
    assert_eq!(
        conversation,
        vec![original.clone(), response.to_assistant_message()]
    );
    assert_eq!(requests.lock().unwrap()[0].messages, vec![original]);
    let appended = events
        .iter()
        .find_map(|event| match event {
            AgentEvent::StepFinished { messages } => Some(messages),
            _ => None,
        })
        .unwrap();
    assert_eq!(appended, &conversation[1..]);
    assert_eq!(
        outcome(&events),
        &RunOutcome::Completed {
            assistant_message: "answer".to_owned(),
            steps_taken: 1,
            structured_output: None,
        }
    );
    assert!(matches!(
        events[events.len() - 2],
        AgentEvent::StepFinished { .. }
    ));
}
