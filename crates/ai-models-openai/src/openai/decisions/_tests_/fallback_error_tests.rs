//! Decisions diagnostics for gateway error bodies.

use ai_interface::{ErrorKind, JudgmentModel};
use serde_json::json;

use crate::OpenAiJudgmentModel;

use super::support::{client, config, predicate, response, spec};

const HEADER_VALUE: &str = r#"fixture"header"#;

#[tokio::test]
async fn json_fallback_redacts_escaped_header_values_in_keys_and_strings() {
    let mut reply = response(400, json!(null));
    reply.body = br#"{
        "message": "gateway rejected fixture\"header",
        "nested": { "fixture\u0022header": ["fixture\u0022header", 7, true, null] }
    }"#
    .to_vec();
    let (http, _) = client(Ok(reply));
    let model = OpenAiJudgmentModel::from_config(
        http,
        spec(),
        config().with_header("X-Fixture-Auth", HEADER_VALUE),
    );

    let error = model.judge(&predicate()).await.unwrap_err();

    assert_eq!(error.core().kind, ErrorKind::InvalidRequest);
    let message = &error.core().message;
    assert!(
        [HEADER_VALUE, r#"fixture\"header"#, r#"fixture\u0022header"#]
            .into_iter()
            .all(|value| !message.contains(value)),
        "fallback exposed a captured header value"
    );
    let expected = r#"{"message":"gateway rejected [redacted]","nested":{"[redacted]":["[redacted]",7,true,null]}}"#;
    assert!(
        message == expected,
        "fallback must preserve compact JSON and redact decoded keys and strings"
    );
}
