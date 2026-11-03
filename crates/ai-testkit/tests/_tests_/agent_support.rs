//! Shared offline agent execution for provider conformance.

use std::sync::Arc;

use ai_interface::{ConversationMessage, DynTool, NoopLogger, ProviderId, ToolMock};
use ai_models_registry::ProviderConfig;
use ai_testkit::{
    provider::{ModelFactory, Provider, ProviderModelFactory},
    scenario::Scenario,
    storage::ScenarioRecording,
};
use ai_tool_calling::{
    Agent, AgentEvent, AgentRequest, InMemoryToolOutputStore, RunOutcome, ToolCallingRuntime,
    ToolOutputPolicy,
};
use json_http::{
    TransportBackedJsonHttpClient,
    test_support::{
        MemoryRecordingLog, Recording, RecordingLog, RecordingTransport, ReplayControl,
        ReplayTransport,
    },
};
use serde_json::json;
use unimock::{MockFn, Unimock, matching};

pub(super) async fn execute_agent(
    provider: Provider,
    scenario: Scenario,
    recording: &ScenarioRecording,
) -> (RunOutcome, Vec<ConversationMessage>, Vec<Recording>) {
    let transport = Arc::new(ReplayTransport::new(recording.exchanges.clone()));
    let log: Arc<dyn RecordingLog + Send + Sync> = Arc::new(MemoryRecordingLog::default());
    let client = Arc::new(TransportBackedJsonHttpClient::new(Arc::new(
        RecordingTransport::new(transport.clone(), log.clone()),
    )));
    let mut config = ProviderConfig::new(
        ProviderId::new(provider.name()),
        "conformance-placeholder-key",
    );
    config.base_url = recording.base_url.clone();
    let model = ProviderModelFactory
        .build(provider, &recording.catalog_model_id, client, config)
        .expect("build registry model");
    let mut request = scenario.request();
    request.controls.generation.effort = recording.effort;
    let tools: Vec<DynTool> = if scenario == Scenario::AgentToolLoop {
        vec![Arc::new(Unimock::new((
            ToolMock::definitions
                .some_call(matching!())
                .returns(request.tools.clone()),
            ToolMock::group_for_tool
                .some_call(matching!("lookup"))
                .returns(None),
            ToolMock::call_with_invocation
                .some_call(matching!((invocation) if invocation.tool_name == "lookup" && invocation.input == json!({"id": 7})))
                .returns(Ok(json!({"value": 7}))),
        ))) ]
    } else {
        Vec::new()
    };
    let agent = ToolCallingRuntime::new(
        model,
        Arc::new(NoopLogger),
        tools,
        Arc::new(InMemoryToolOutputStore::new()),
        ToolOutputPolicy::default(),
    )
    .expect("build conformance agent");
    let mut conversation = request.messages;
    let initial_len = conversation.len();
    let mut outcome = None;
    {
        let mut stream = agent.run(
            &mut conversation,
            AgentRequest {
                system_prompt: request.system_prompt,
                controls: request.controls,
                max_steps: Some(2),
                final_answer_schema: request.response_schema,
            },
        );
        while let Some(event) = stream.next().await.unwrap_or_else(|error| {
            panic!("{}/{} agent run: {error}", provider.name(), scenario.name())
        }) {
            if let AgentEvent::Finished(finished) = event {
                outcome = Some(finished);
            }
        }
    }
    assert_eq!(transport.remaining(), 0, "all agent exchanges consumed");
    let steps_taken = if scenario == Scenario::AgentToolLoop {
        2
    } else {
        1
    };
    let outcome = outcome.expect("agent terminal outcome");
    assert_eq!(
        outcome,
        RunOutcome::Completed {
            assistant_message: "Done".to_owned(),
            structured_output: None,
            steps_taken,
        },
        "{}/{} agent outcome",
        provider.name(),
        scenario.name(),
    );
    (
        outcome,
        conversation[initial_len..].to_vec(),
        log.snapshot(),
    )
}
