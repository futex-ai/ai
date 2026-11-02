//! Request comparison diagnostics for outbound serialization regressions.

use std::collections::BTreeMap;

use json_http::test_support::{
    RecordedBody, RecordedMethod, RecordedRequest, RecordedResponse, Recording,
};
use serde_json::json;

use crate::{error::Error, provider::Provider, scenario::Scenario};

use super::compare_requests;

#[test]
fn body_difference_names_provider_scenario_and_exchange() {
    let exchange = Recording {
        request: RecordedRequest {
            method: RecordedMethod::Post,
            url: "https://api.openai.com/v1/responses".to_owned(),
            headers: BTreeMap::from([("Authorization".to_owned(), "[redacted]".to_owned())]),
            body: Some(RecordedBody::Json(json!({"tool_choice": "none"}))),
        },
        response: RecordedResponse::Json {
            headers: Default::default(),
            status: 200,
            body: json!({}),
        },
    };
    let expected = vec![exchange.clone(), exchange];
    let mut actual = expected.clone();
    actual[1].request.body = Some(RecordedBody::Json(json!({"tool_choice": "auto"})));

    let error = compare_requests(Provider::Openai, Scenario::ToolLoop, &expected, &actual)
        .expect_err("a changed body field must fail");
    let Error::RequestMismatch {
        provider,
        scenario,
        exchange_index,
        expected,
        actual,
    } = &error
    else {
        panic!("unexpected error: {error}");
    };
    assert_eq!(*provider, Provider::Openai);
    assert_eq!(*scenario, Scenario::ToolLoop);
    assert_eq!(*exchange_index, 1);
    assert!(expected.contains("\"tool_choice\": \"none\""));
    assert!(actual.contains("\"tool_choice\": \"auto\""));
    let message = error.to_string();
    assert!(message.contains("openai/tool_loop exchange index 1 request mismatch"));
    assert!(message.contains("\nexpected:\n{\n"));
    assert!(message.contains("\nactual:\n{\n"));
}
