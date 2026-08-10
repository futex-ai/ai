//! JSON-RPC response-shape classifier tests.

use serde_json::{Value, json};

use crate::protocol::{JsonRpcMessageKind, classify_message};

#[test]
fn rejects_mixed_result_error_members_for_every_id_state() {
    let message = json!({
        "jsonrpc": "2.0",
        "result": {"ok": true},
        "error": {"code": -32603, "message": "failure"}
    });
    let ids = [
        Some(json!("server-1")),
        Some(json!(7)),
        Some(json!(1.5)),
        Some(Value::Null),
        None,
        Some(json!(true)),
        Some(json!({})),
        Some(json!([])),
    ];

    for id in ids {
        assert!(matches!(
            classify_message(&with_optional_id(&message, id)),
            JsonRpcMessageKind::Invalid
        ));
    }
}

#[test]
fn rejects_mixed_result_error_members_by_presence_not_value() {
    let messages = [
        json!({
            "jsonrpc": "2.0",
            "id": 1,
            "result": null,
            "error": {"code": -32603, "message": "failure"}
        }),
        json!({
            "jsonrpc": "2.0",
            "id": 1,
            "result": {"ok": true},
            "error": "malformed"
        }),
    ];

    for message in messages {
        assert!(matches!(
            classify_message(&message),
            JsonRpcMessageKind::Invalid
        ));
    }
}

fn with_optional_id(message: &Value, id: Option<Value>) -> Value {
    let mut message = message.clone();
    if let Some(id) = id {
        message.as_object_mut().unwrap().insert("id".to_owned(), id);
    }
    message
}
