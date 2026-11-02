//! Native item identity, tool lifecycle, and terminal validation tests.

use ai_interface::{
    ConversationMessage, FinishReason, Model, ModelRequest, StructuredOutputSchema,
};
use ai_models_core::test_support::collect_model_events;
use serde_json::{Value, json};

use super::stream_support::{custom_spec, event, terminal_event};
use super::{OpenAiModel, stream_support};

#[tokio::test]
async fn emits_indexed_interleaved_tools_reasoning_and_refusal_before_terminal_metadata() {
    let events = vec![
        native(
            "response.reasoning_summary_text.delta",
            json!({"output_index": 0, "summary_index": 0, "delta": "thi"}),
        ),
        native(
            "response.reasoning_text.delta",
            json!({"output_index": 0, "content_index": 0, "delta": "nk"}),
        ),
        native(
            "response.output_text.delta",
            json!({"output_index": 1, "content_index": 0, "delta": "Hello"}),
        ),
        native(
            "response.refusal.delta",
            json!({"output_index": 1, "content_index": 1, "delta": " denied"}),
        ),
        native(
            "response.output_item.added",
            json!({"output_index": 2, "item": call("fc_1", "call_1", "lookup", "")}),
        ),
        native(
            "response.function_call_arguments.delta",
            json!({"output_index": 2, "delta": "{\"key\":"}),
        ),
        native(
            "response.output_item.added",
            json!({"output_index": 3, "item": call("fc_2", "call_2", "status", "")}),
        ),
        native(
            "response.function_call_arguments.delta",
            json!({"output_index": 3, "delta": "{}"}),
        ),
        native(
            "response.function_call_arguments.done",
            json!({"output_index": 3, "arguments": "{}"}),
        ),
        native(
            "response.function_call_arguments.delta",
            json!({"output_index": 2, "delta": "\"root\"}"}),
        ),
        native(
            "response.output_item.done",
            json!({"output_index": 2, "item": call("fc_1", "call_1", "lookup", "{\"key\":\"root\"}")}),
        ),
        native(
            "response.output_item.done",
            json!({"output_index": 3, "item": call("fc_2", "call_2", "status", "{}")}),
        ),
        terminal_event("response.completed", rich_terminal()),
    ];
    let (client, _) = stream_support::recording_streaming_client(vec![events]);
    let model = OpenAiModel::new(client, custom_spec("gpt-5.5"), "key");
    let mut stream = model.stream(&request()).await.expect("stream should open");
    let mut actual = Vec::new();
    while let Some(event) = stream.next().await.expect("event should succeed") {
        actual.push(serde_json::to_value(event).expect("event should serialize"));
    }

    assert_eq!(
        actual,
        vec![
            json!({"ReasoningDelta": {"part_index": 0, "delta": "thi"}}),
            json!({"ReasoningDelta": {"part_index": 0, "delta": "nk"}}),
            json!({"TextDelta": {"part_index": 1, "delta": "Hello"}}),
            json!({"TextDelta": {"part_index": 2, "delta": " denied"}}),
            json!({"ToolCallStart": {"part_index": 3, "id": "call_1", "name": "lookup"}}),
            json!({"ToolCallArgumentsDelta": {"part_index": 3, "delta": "{\"key\":"}}),
            json!({"ToolCallStart": {"part_index": 4, "id": "call_2", "name": "status"}}),
            json!({"ToolCallArgumentsDelta": {"part_index": 4, "delta": "{}"}}),
            json!({"ToolCallEnd": {"part_index": 4,"provider_state":{"provider":"openai","model":"gpt-5.5","version":1,"payload":{"id":"fc_2","call_id":"call_2","name":"status","arguments":"{}"}}}}),
            json!({"ToolCallArgumentsDelta": {"part_index": 3, "delta": "\"root\"}"}}),
            json!({"ToolCallEnd": {"part_index": 3,"provider_state":{"provider":"openai","model":"gpt-5.5","version":1,"payload":{"id":"fc_1","call_id":"call_1","name":"lookup","arguments":"{\"key\":\"root\"}"}}}}),
            json!({"ReasoningDelta":{"part_index":0,"delta":"","provider_state":{"provider":"openai","model":"gpt-5.5","version":1,"payload":{"id":"rs_1","summary":[{"type":"summary_text","text":"think"}],"encrypted_content":"encrypted"}}}}),
            json!({"TextDelta":{"part_index":1,"delta":"","provider_state":{"provider":"openai","model":"gpt-5.5","version":1,"payload":{"phase":"final_answer"}}}}),
            json!({"Usage": {"usage": {
                "input_tokens": 8, "output_tokens": 4, "cached_input_tokens": 2,
                "reasoning_tokens": 1, "total_tokens": 15, "estimated_cost_microusd": 0
            }}}),
            json!({"Finish": {
                "finish_reason": "ToolCalls", "provider": "openai", "model_id": "gpt-5.5",
                "catalog_model_id": "openai/gpt-5.5", "effort": "disabled",

            }}),
        ]
    );
    assert!(
        stream
            .next()
            .await
            .expect("closed stream should stay closed")
            .is_none()
    );
}

#[tokio::test]
async fn emits_terminal_only_content_without_duplicating_streamed_text() {
    let events = vec![
        native(
            "response.output_text.delta",
            json!({"output_index": 0, "content_index": 0, "delta": "Hello"}),
        ),
        terminal_event(
            "response.completed",
            json!({
                "status": "completed", "output": [
                    {"type": "message", "content": [
                        {"type": "output_text", "text": "Hello"}, {"type": "refusal", "refusal": " denied"}
                    ]},
                    {"type": "reasoning", "id": "rs_1", "summary": [{"type": "summary_text", "text": "think"}]}
                ]
            }),
        ),
    ];
    let (client, _) = stream_support::recording_streaming_client(vec![events]);
    let model = OpenAiModel::new(client, custom_spec("gpt-5.5"), "key");
    let (response, events) = collect_model_events(&model, &request()).await;
    let content = events
        .into_iter()
        .filter(|event| event.get("TextDelta").is_some() || event.get("ReasoningDelta").is_some())
        .collect::<Vec<_>>();

    assert_eq!(
        content,
        vec![
            json!({"TextDelta": {"part_index": 0, "delta": "Hello"}}),
            json!({"TextDelta": {"part_index": 1, "delta": " denied"}}),
            json!({"ReasoningDelta": {"part_index": 2, "delta": "think","provider_state":{"provider":"openai","model":"gpt-5.5","version":1,"payload":{"id":"rs_1","summary":[{"type":"summary_text","text":"think"}]}}}}),
        ]
    );
    let response = response.expect("terminal-only parts should fold");
    assert_eq!(response.text(), "Hello denied");
    assert_eq!(response.finish_reason, FinishReason::Filtered);

    let mut actual = Vec::new();
    let mut expected = Vec::new();
    for (case, delta, text, deltas) in [
        ("item start", Value::Null, "Hello", json!(["Hello"])),
        (
            "anonymous",
            json!({"delta":"Hello"}),
            "Hello",
            json!(["Hello"]),
        ),
        (
            "item id",
            json!({"item_id":"msg","delta":"Hello"}),
            "Hello",
            json!(["Hello"]),
        ),
        (
            "partial",
            json!({"output_index":0,"delta":"Hel"}),
            "Hello",
            json!(["Hel", "lo"]),
        ),
        (
            "partial anonymous",
            json!({"delta":"Hel"}),
            "Hello",
            json!(["Hel", "lo"]),
        ),
        (
            "divergent",
            json!({"output_index":0,"delta":"Other"}),
            "Other",
            json!(["Other"]),
        ),
    ] {
        let message = json!({"type":"message","id":"msg","phase":"commentary","content":[{"type":"output_text","text":"Hello"}]});
        let reasoning = json!({"type":"reasoning","id":"rs_1","summary":[{"type":"summary_text","text":"think"}]});
        let mut events = vec![
            native(
                "response.output_item.added",
                json!({"output_index":0,"item":{"type":"message","id":"msg","phase":"commentary","content":[]}}),
            ),
            native(
                "response.output_item.added",
                json!({"output_index":1,"item":{"type":"reasoning","id":"rs_1","summary":[]}}),
            ),
        ];
        if !delta.is_null() {
            events.push(native("response.output_text.delta", delta));
        }
        events.extend([
            native(
                "response.output_item.done",
                json!({"output_index":0,"item":message}),
            ),
            native(
                "response.output_item.done",
                json!({"output_index":1,"item":reasoning}),
            ),
            terminal_event(
                "response.completed",
                json!({"status":"completed","output":[message,reasoning]}),
            ),
        ]);
        let (client, _) = stream_support::recording_streaming_client(vec![events]);
        let model = OpenAiModel::new(client, custom_spec("gpt-5.5"), "key");
        let (response, events) = collect_model_events(&model, &request()).await;
        let text_deltas = events
            .iter()
            .filter_map(|event| {
                event
                    .get("TextDelta")?
                    .get("delta")?
                    .as_str()
                    .filter(|delta| !delta.is_empty())
            })
            .collect::<Vec<_>>();
        let reasoning = events
            .iter()
            .filter_map(|event| event.get("ReasoningDelta")?.get("delta")?.as_str())
            .collect::<String>();
        actual.push(json!({"case":case,"text":response.unwrap().text(),"deltas":text_deltas,"reasoning":reasoning}));
        expected.push(json!({"case":case,"text":text,"deltas":deltas,"reasoning":"think"}));
    }
    assert_eq!(actual, expected);
}

#[tokio::test]
async fn validates_terminal_tool_and_schema_output_before_usage_and_closes() {
    let cases = [
        (
            json!({"status": "completed", "output": [call("fc_1", "call_1", "lookup", "{")]}),
            None,
        ),
        (
            json!({"status": "completed", "output": [{"type": "message", "content": [{"type": "output_text", "text": "{}"}]}]}),
            Some(StructuredOutputSchema {
                name: "summary".to_owned(),
                schema: json!({"type": "object", "required": ["summary"]}),
            }),
        ),
    ];
    for (body, schema) in cases {
        let (client, _) = stream_support::recording_streaming_client(vec![vec![terminal_event(
            "response.completed",
            body,
        )]]);
        let model = OpenAiModel::new(client, custom_spec("gpt-5.5"), "key");
        let mut request = request();
        request.response_schema = schema;
        let mut stream = model.stream(&request).await.expect("stream should open");
        let result = stream.next().await;
        assert!(
            matches!(result, Err(ref error) if error.core().kind == ai_interface::ErrorKind::InvalidOutput)
        );
        assert!(
            stream
                .next()
                .await
                .expect("failed stream should stay closed")
                .is_none()
        );
    }
}

fn native(kind: &str, mut fields: Value) -> stream_support::StreamItem {
    fields["type"] = Value::String(kind.to_owned());
    event(kind, fields)
}

fn call(id: &str, call_id: &str, name: &str, arguments: &str) -> Value {
    json!({"type": "function_call", "id": id, "call_id": call_id, "name": name, "arguments": arguments})
}

fn rich_terminal() -> Value {
    json!({"status": "completed", "output": [
        {"type": "reasoning", "id": "rs_1", "summary": [{"type": "summary_text", "text": "think"}], "encrypted_content": "encrypted"},
        {"type": "message", "phase": "final_answer", "content": [{"type": "output_text", "text": "Hello"}, {"type": "refusal", "refusal": " denied"}]},
        call("fc_1", "call_1", "lookup", "{\"key\":\"root\"}"), call("fc_2", "call_2", "status", "{}")
    ], "usage": {"input_tokens": 10, "input_tokens_details": {"cached_tokens": 2}, "output_tokens": 5,
        "output_tokens_details": {"reasoning_tokens": 1}, "total_tokens": 15}})
}

fn request() -> ModelRequest {
    ModelRequest {
        system_prompt: "system".to_owned(),
        messages: vec![ConversationMessage::user("hello")],
        tools: Vec::new(),
        response_schema: None,
        controls: Default::default(),
    }
}
