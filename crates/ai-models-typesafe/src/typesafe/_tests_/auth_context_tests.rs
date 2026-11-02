//! Typed answer fields and dynamic auth remain secret-safe within each judgment call.

use std::{collections::BTreeMap, sync::Arc, time::Duration};

use ai_interface::{
    ErrorKind, JudgmentAnswerProblem, JudgmentError, JudgmentModel, JudgmentQuestion,
    JudgmentRequest,
};
use json_http::JsonHttpAuthMock;
use serde_json::json;
use unimock::{MockFn, Unimock, matching};

use super::{
    TypeSafeJudgmentModel,
    error_context_tests::headers,
    test_support::{json_response, scripted_http_client, simple_request},
};

#[tokio::test]
async fn typed_answer_ids_and_labels_and_rotating_auth_stay_safe_and_call_scoped() {
    let auth = Unimock::new(JsonHttpAuthMock::apply_headers.stub(|calls| {
        calls
            .call(matching!(_))
            .answers(&|_, headers: &mut BTreeMap<String, String>| {
                headers.insert("X-Auth".to_owned(), "Scheme prefix first-secret".to_owned());
                Ok(())
            })
            .once()
            .then()
            .answers(&|_, headers: &mut BTreeMap<String, String>| {
                headers.insert(
                    "X-Auth".to_owned(),
                    "Scheme prefix second-secret".to_owned(),
                );
                Ok(())
            })
            .once()
            .then()
            .answers(&|_, _| Err(json_http::Error::auth("undiscovered-secret")))
            .once();
    }));
    let mut first = json_response(
        200,
        json!({"model":"jev-1.13.0","answers":{"q-first-secret":{"type":"choice","choice":"first-secret","probabilities":{"billing":1.0},"confidence":1.0}}}),
    );
    first.headers = headers("req-first-secret", "3");
    let second = json_http::Error::HttpStatus {
        status: 403,
        headers: Box::new(headers("req-inner", "1")),
        body: json!({"detail":"first-secret second-secret"}),
    }
    .with_response_headers(headers("req-second-secret", "9"));
    let (http, requests) = scripted_http_client(vec![Ok(first), Err(second)]);
    let model = TypeSafeJudgmentModel::with_auth(http, "jev-custom", Arc::new(auth))
        .with_clock(Arc::new(Unimock::new(())));
    let input = JudgmentRequest {
        state: "ticket".into(),
        questions: BTreeMap::from([(
            "q-first-secret".to_owned(),
            JudgmentQuestion::choice("Choose", [("billing", None)]),
        )]),
    };
    let first = model.judge(&input).await.unwrap_err();
    let JudgmentError::InvalidAnswer { core, id, problem } = &first else {
        panic!("typed answer fields lost")
    };
    assert_eq!(core.kind, ErrorKind::InvalidOutput);
    assert_eq!(core.provider, "typesafe");
    assert_eq!(core.model_id, "jev-custom");
    assert_eq!(id, "q-[redacted]");
    assert_eq!(
        problem,
        &JudgmentAnswerProblem::UnknownOption {
            label: "[redacted]".to_owned()
        }
    );
    assert_eq!(core.provider_request_id.as_deref(), Some("req-[redacted]"));
    assert_eq!(core.retry_after, Some(Duration::from_secs(3)));
    assert!(!first.to_string().contains("first-secret"));
    assert!(!format!("{first:?}").contains("first-secret"));
    let second = model.judge(&simple_request()).await.unwrap_err();
    assert_eq!(second.core().kind, ErrorKind::RejectedKey);
    assert_eq!(second.core().message, "first-secret [redacted]");
    assert_eq!(
        second.core().provider_request_id.as_deref(),
        Some("req-[redacted]")
    );
    assert_eq!(second.core().retry_after, Some(Duration::from_secs(9)));
    assert!(!second.to_string().contains("second-secret"));
    assert!(!format!("{second:?}").contains("second-secret"));
    let hook = model.judge(&simple_request()).await.unwrap_err();
    assert_eq!(hook.core().kind, ErrorKind::Transient);
    assert_eq!(hook.core().message, "authentication hook failed");
    assert_eq!(hook.core().provider_request_id, None);
    assert_eq!(hook.core().retry_after, None);
    assert!(matches!(&hook, JudgmentError::Failure { .. }));
    assert!(!hook.to_string().contains("undiscovered-secret"));
    assert!(!format!("{hook:?}").contains("undiscovered-secret"));
    let requests = requests.lock().unwrap();
    assert_eq!(requests.len(), 2);
    assert_eq!(
        requests[0].headers.get("X-Auth").map(String::as_str),
        Some("Scheme prefix first-secret")
    );
    assert_eq!(
        requests[1].headers.get("X-Auth").map(String::as_str),
        Some("Scheme prefix second-secret")
    );
}
