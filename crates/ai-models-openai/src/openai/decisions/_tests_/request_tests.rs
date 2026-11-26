//! Decisions request wire mapping through the injected transport.

use std::{collections::BTreeMap, time::Duration};

use ai_interface::{
    ConversationMessage, ConversationPart, ConversationRole, JudgmentConditionCriteria,
    JudgmentContent, JudgmentDescription, JudgmentInput, JudgmentJson, JudgmentLevel,
    JudgmentModel, JudgmentPart, JudgmentQuestion, JudgmentRequest, ProviderState,
};
use json_http::JsonHttpMethod;
use serde_json::{Map, json};

use crate::PROVIDER_ID;

use super::support::{
    API_KEY, MODEL_ID, model, predicate, predicate_response, request_body, sent_request,
};

#[tokio::test]
async fn single_text_input_sends_one_predicate_request() {
    let (model, requests) = model(predicate_response());
    let mut request = predicate();
    request.input = "  Is there an urgent ticket?  ".into();
    let result = model.judge(&request).await;

    let http = sent_request(&requests);
    assert_eq!(http.method, JsonHttpMethod::Post);
    assert_eq!(http.url, "https://api.openai.com/v1/decisions");
    assert_eq!(http.timeout, Duration::from_secs(60));
    assert_eq!(http.idle_timeout, None);
    assert_eq!(
        http.headers.get("Authorization").map(String::as_str),
        Some(format!("Bearer {API_KEY}").as_str())
    );
    assert_eq!(
        request_body(&http),
        json!({
            "model": MODEL_ID,
            "input": "  Is there an urgent ticket?  ",
            "questions": [{"name": "q0", "type": "predicate", "instructions": "Is this urgent?"}]
        })
    );
    result.unwrap();
}

#[tokio::test]
async fn multimodal_content_and_questions_use_decisions_wire_shapes() {
    let response = json!({"model":"decisions-release","answers":[
        {"type":"refusal","name":"q0"},
        {"type":"refusal","name":"q1"},
        {"type":"refusal","name":"q2"}
    ]});
    let (model, requests) = model(response);
    let request = JudgmentRequest {
        input: JudgmentContent {
            parts: vec![
                JudgmentPart::Text {
                    text: "Read the ticket.".to_owned(),
                },
                JudgmentPart::Json {
                    value: JudgmentJson::Object(Map::from_iter([(
                        "ticket".to_owned(),
                        json!("Late"),
                    )])),
                },
                JudgmentPart::Json {
                    value: JudgmentJson::Array(vec![json!({"id": 1}), json!({"id": 2})]),
                },
                JudgmentPart::Json {
                    value: JudgmentJson::Array(vec![json!(3), json!({"x": true})]),
                },
                image("AQID"),
            ],
        }
        .into(),
        questions: BTreeMap::from([
            (
                "sort-last".to_owned(),
                JudgmentQuestion::score(
                    "Rate it.",
                    [
                        JudgmentLevel::new(
                            "low",
                            Some(JudgmentDescription::try_from(json!({"delay": 1})).unwrap()),
                        ),
                        JudgmentLevel::from("high"),
                    ],
                ),
            ),
            (
                "sort-first".to_owned(),
                JudgmentQuestion::choice(
                    JudgmentContent {
                        parts: vec![
                            JudgmentPart::Text {
                                text: "Compare.".to_owned(),
                            },
                            image("BAUG"),
                            JudgmentPart::Json {
                                value: JudgmentJson::Array(vec![]),
                            },
                        ],
                    },
                    [("technical", None), ("billing", Some("Payouts".into()))],
                ),
            ),
            (
                "sort-in-middle".to_owned(),
                JudgmentQuestion::condition(
                    "Is it urgent?",
                    Some(JudgmentConditionCriteria::new(
                        Some("Act today".into()),
                        Some(JudgmentDescription::try_from(json!(["resolved", false])).unwrap()),
                    )),
                ),
            ),
        ]),
    };
    let result = model.judge(&request).await;

    assert_eq!(
        request_body(&sent_request(&requests)),
        json!({
            "model": MODEL_ID,
            "input": [{"role":"user","content":[
                {"type":"input_text","text":"Read the ticket."},
                {"type":"input_json","data":{"ticket":"Late"}},
                {"type":"input_json","data":[{"id":1},{"id":2}]},
                {"type":"input_text","text":"[3,{\"x\":true}]"},
                {"type":"input_image","image_url":"data:image/png;base64,AQID"}
            ]}],
            "questions": [
                {"name":"q0","type":"choice","instructions":[
                    {"type":"input_text","text":"Compare."},
                    {"type":"input_image","image_url":"data:image/png;base64,BAUG"},
                    {"type":"input_json","data":[]}
                ],"choices":[{"value":"technical"},{"value":"billing","description":"Payouts"}]},
                {"name":"q1","type":"choice","instructions":"Is it urgent?","choices":[
                    {"value":true,"description":"Act today"},
                    {"value":false,"description":"[\"resolved\",false]"}
                ]},
                {"name":"q2","type":"score","instructions":"Rate it.","levels":[
                    {"label":"low","description":"{\"delay\":1}"},{"label":"high"}
                ]}
            ]
        })
    );
    result.unwrap();
}

#[tokio::test]
async fn conversation_sends_portable_messages_and_tool_items_in_order() {
    let mut user = ConversationMessage::user_with_parts(vec![
        ConversationPart::text("Check the invoice.").with_provider_state(ProviderState {
            provider: PROVIDER_ID,
            model: MODEL_ID.to_owned(),
            version: 99,
            payload: Map::default(),
        }),
        ConversationPart::image("image/png", "AQID"),
    ]);
    user.name = Some("buyer".to_owned());
    let messages = vec![
        user,
        ConversationMessage::new(
            ConversationRole::Assistant,
            vec![
                ConversationPart::text("I'll look it up."),
                ConversationPart::reasoning("omit reasoning"),
                ConversationPart::ToolCall {
                    call_id: "lookup-1".to_owned(),
                    tool_name: "lookup".to_owned(),
                    arguments: json!({"invoice": 3}),
                    operation_id: Some("omit-operation".to_owned()),
                    provider_state: None,
                },
                ConversationPart::text("Checking."),
            ],
        ),
        ConversationMessage::tool("overdue", "lookup", "lookup-1"),
        ConversationMessage::user_with_parts(vec![]),
    ];

    for (system, developer_message) in [
        (
            "  Check the final message.  ",
            Some(json!({"role":"developer","content":"  Check the final message.  "})),
        ),
        (" \t ", None),
    ] {
        let (model, requests) = model(predicate_response());
        let mut request = predicate();
        request.input = JudgmentInput::Conversation {
            system: Some(system.to_owned()),
            messages: messages.clone(),
        };
        let result = model.judge(&request).await;
        let mut expected = Vec::from_iter(developer_message);
        expected.extend([
            json!({"role":"user","content":[
                {"type":"input_text","text":"Check the invoice."},
                {"type":"input_image","image_url":"data:image/png;base64,AQID"}
            ]}),
            json!({"role":"assistant","content":"I'll look it up."}),
            json!({"type":"function_call","call_id":"lookup-1","name":"lookup","arguments":"{\"invoice\":3}"}),
            json!({"role":"assistant","content":"Checking."}),
            json!({"type":"function_call_output","call_id":"lookup-1","output":"overdue"}),
            json!({"role":"user","content":[]}),
        ]);

        assert_eq!(
            request_body(&sent_request(&requests))["input"],
            json!(expected)
        );
        result.unwrap();
    }
}

fn image(data_base64: &str) -> JudgmentPart {
    JudgmentPart::Image {
        mime_type: "image/png".to_owned(),
        data_base64: data_base64.to_owned(),
    }
}
