//! Deferred xAI results expose all tool data before finish filtering.

use std::sync::Arc;

use ai_interface::{
    ConversationMessage, ModelCallControls, ModelCompletionMode, ModelExecutionControls,
    ModelRequest, ModelResponse,
};
use json_http::{
    JsonHttpResponse, JsonHttpTransportMock, StaticHeaderAuth, TransportBackedJsonHttpClient,
};
use serde_json::{Value, json};
use unimock::{MockFn, Unimock, matching};

use crate::stream_test_support::{observe, portable_events};

use super::{XaiModel, deferred::DeferredRuntimeMock, test_support::custom_spec};

#[tokio::test]
async fn deferred_result_emits_reasoning_tools_and_terminal_metadata() {
    for (reason, legacy, arguments, retained) in [
        ("tool_calls", false, "{ \"id\": 7 }", 1),
        ("function_call", true, "{ \"id\": 7 }", 1),
        ("length", false, "{ \"id\":", 0),
        ("length", true, "{ \"id\":", 0),
    ] {
        let (response, events) = complete_result(reason, legacy, arguments).await;
        let events = portable_events(&events);
        assert_eq!(events.len(), 7, "{reason}, legacy={legacy}");
        assert_eq!(
            events[0],
            json!({"ReasoningDelta": {"part_index": 0, "delta": "think"}})
        );
        assert_eq!(
            events[1],
            json!({"TextDelta": {"part_index": 1, "delta": "checking"}})
        );
        let start = &events[2]["ToolCallStart"];
        assert_eq!(start["part_index"], 2);
        assert_eq!(start["name"], "lookup");
        let id = start["id"].as_str().expect("tool identifier");
        let identity = if legacy {
            id.strip_prefix("xai_legacy_function_call:")
                .map(|_| "legacy")
        } else {
            Some(id)
        };
        assert_eq!(identity, Some(if legacy { "legacy" } else { "call-1" }));
        assert_eq!(
            start["operation_id"],
            if legacy {
                start["id"].clone()
            } else {
                Value::Null
            }
        );
        assert_eq!(
            events[3],
            json!({"ToolCallArgumentsDelta": {"part_index": 2, "delta": arguments}})
        );
        assert_eq!(events[4], json!({"ToolCallEnd": {"part_index": 2}}));
        assert_eq!(events[5]["Usage"]["usage"]["total_tokens"], 5);
        assert_eq!(
            events[6]["Finish"]["finish_reason"],
            if reason == "length" {
                "Truncated"
            } else {
                "ToolCalls"
            }
        );
        assert_eq!(response.tool_calls().len(), retained);
        assert_eq!(
            response.tool_calls().first().map(|call| &call.input),
            (retained == 1).then_some(&json!({"id": 7}))
        );
    }
}

async fn complete_result(
    reason: &str,
    legacy: bool,
    arguments: &str,
) -> (ModelResponse, Vec<Value>) {
    let function = json!({"name": "lookup", "arguments": arguments});
    let mut message = json!({"reasoning_content": "think", "content": "checking"});
    if legacy {
        message["function_call"] = function;
    } else {
        message["tool_calls"] = json!([{"id": "call-1", "function": function}]);
    }
    let body = json!({
        "choices": [{"message": message, "finish_reason": reason}],
        "usage": {"prompt_tokens": 2, "completion_tokens": 3, "total_tokens": 5}
    });
    let transport = Arc::new(Unimock::new((
        JsonHttpTransportMock::execute
            .next_call(matching!(_))
            .answers(&|_, _| {
                Ok(JsonHttpResponse {
                    headers: Default::default(),
                    status: 200,
                    body: json!({"request_id": "accepted"}),
                })
            }),
        JsonHttpTransportMock::execute
            .next_call(matching!(_))
            .answers_arc(Arc::new(move |_, _| {
                Ok(JsonHttpResponse {
                    headers: Default::default(),
                    status: 200,
                    body: body.clone(),
                })
            })),
    )));
    let runtime = Arc::new(Unimock::new(
        DeferredRuntimeMock::now
            .each_call(matching!())
            .returns(tokio::time::Instant::now()),
    ));
    let model = XaiModel::with_catalog_auth_and_runtime(
        Arc::new(TransportBackedJsonHttpClient::new(transport)),
        custom_spec("grok-4"),
        Arc::new(StaticHeaderAuth::bearer_token("key")),
        runtime,
    );
    let request = ModelRequest {
        system_prompt: String::new(),
        messages: vec![ConversationMessage::user("hello")],
        tools: Vec::new(),
        response_schema: None,
        controls: ModelCallControls {
            execution: ModelExecutionControls {
                completion_mode: ModelCompletionMode::RequireDeferred,
                ..Default::default()
            },
            ..Default::default()
        },
    };
    let mut events = Vec::new();
    let response = observe(&model, &request, &mut events)
        .await
        .expect("valid deferred result");
    (response, events)
}
