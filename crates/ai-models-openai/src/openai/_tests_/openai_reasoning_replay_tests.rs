//! Tests for OpenAI Responses reasoning replay request mapping.

use std::sync::{Arc, Mutex};

use ai_interface::{
    ConversationMessage, Model, ModelRequest, OpenAiReasoningSummary, ProviderConversationItem,
    ToolCall,
};
use json_http::{
    JsonHttpClient, JsonHttpRequest, JsonHttpResponse, JsonHttpTransportMock,
    TransportBackedJsonHttpClient,
};
use serde_json::json;
use unimock::{MockFn, Unimock, matching};

use super::OpenAiModel;

type RecordedRequests = Arc<Mutex<Vec<JsonHttpRequest>>>;

#[tokio::test]
async fn replays_openai_reasoning_context_before_tool_outputs() {
    let (http_client, requests) = recording_http_client(openai_text_response("Done"));
    let model = OpenAiModel::new(http_client, "gpt-5.5", "sk-openai");
    let assistant = ConversationMessage::assistant_with_provider_context(
        "",
        vec![ToolCall {
            id: "call_1".to_owned(),
            name: "memory_read".to_owned(),
            input: json!({"path": "root"}),
            operation_id: None,
        }],
        vec![ProviderConversationItem::OpenAiReasoning {
            id: "rs_1".to_owned(),
            summary: vec![OpenAiReasoningSummary {
                kind: "summary_text".to_owned(),
                text: "Need the memory tool.".to_owned(),
            }],
            encrypted_content: Some("encrypted-reasoning".to_owned()),
        }],
    );

    model
        .complete(&ModelRequest {
            system_prompt: "system".to_owned(),
            messages: vec![
                ConversationMessage::user("start"),
                assistant,
                ConversationMessage::tool("{\"ok\":true}", "memory_read", "call_1"),
            ],
            tools: Vec::new(),
            response_schema: None,
        })
        .await
        .expect("OpenAI response should parse");

    let requests = requests
        .lock()
        .expect("requests lock should not be poisoned");
    let input = &requests[0].body.as_ref().expect("body present")["input"];
    assert_eq!(input[1]["type"], "reasoning");
    assert_eq!(input[1]["id"], "rs_1");
    assert_eq!(input[1]["summary"][0]["type"], "summary_text");
    assert_eq!(input[1]["encrypted_content"], "encrypted-reasoning");
    assert_eq!(input[2]["type"], "function_call");
    assert_eq!(input[2]["call_id"], "call_1");
    assert_eq!(input[3]["type"], "function_call_output");
    assert_eq!(input[3]["call_id"], "call_1");
}

fn recording_http_client(
    response: JsonHttpResponse<serde_json::Value>,
) -> (Arc<dyn JsonHttpClient>, RecordedRequests) {
    let requests = Arc::new(Mutex::new(Vec::new()));
    let transport = Arc::new(Unimock::new(
        JsonHttpTransportMock::execute
            .each_call(matching!(_))
            .answers_arc({
                let requests = requests.clone();
                Arc::new(move |_, request: &JsonHttpRequest| {
                    requests
                        .lock()
                        .expect("requests lock should not be poisoned")
                        .push(request.clone());
                    Ok(response.clone())
                })
            }),
    ));

    (
        Arc::new(TransportBackedJsonHttpClient::new(transport)),
        requests,
    )
}

fn openai_text_response(text: &str) -> JsonHttpResponse<serde_json::Value> {
    JsonHttpResponse {
        status: 200,
        body: json!({
            "status": "completed",
            "output": [{
                "type": "message",
                "role": "assistant",
                "content": [{ "type": "output_text", "text": text }]
            }],
            "usage": {
                "input_tokens": 120,
                "output_tokens": 32,
                "total_tokens": 152
            }
        }),
    }
}
