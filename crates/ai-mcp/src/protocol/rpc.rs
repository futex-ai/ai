//! JSON-RPC message construction and classification.

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value, json};

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(untagged)]
/// JSON-RPC request identifier, preserved without string/number coercion.
pub enum McpRequestId {
    /// Numeric request identifier.
    Number(serde_json::Number),
    /// String request identifier.
    String(String),
}

#[derive(Clone, Debug, Deserialize)]
pub(crate) struct JsonRpcErrorBody {
    pub(crate) code: i64,
    pub(crate) message: String,
    pub(crate) data: Option<Value>,
}

pub(crate) enum JsonRpcMessageKind {
    Response {
        id: McpRequestId,
        result: Value,
    },
    Error {
        id: Option<McpRequestId>,
        error: JsonRpcErrorBody,
    },
    Request {
        id: McpRequestId,
        method: String,
    },
    Notification {
        method: String,
    },
    Invalid,
}

enum JsonRpcIdState {
    Absent,
    Null,
    Valid(McpRequestId),
    Invalid,
}

pub(crate) fn request(id: u64, method: &str, params: Value) -> Value {
    json!({
        "jsonrpc": "2.0",
        "id": id,
        "method": method,
        "params": params
    })
}

pub(crate) fn notification(method: &str, params: Value) -> Value {
    json!({
        "jsonrpc": "2.0",
        "method": method,
        "params": params
    })
}

pub(crate) fn success_response(id: &McpRequestId, result: Value) -> Value {
    json!({"jsonrpc": "2.0", "id": id, "result": result})
}

pub(crate) fn error_response(id: &McpRequestId, code: i64, message: &str) -> Value {
    json!({
        "jsonrpc": "2.0",
        "id": id,
        "error": {"code": code, "message": message}
    })
}

pub(crate) fn classify_message(value: &Value) -> JsonRpcMessageKind {
    let Some(object) = value.as_object() else {
        return JsonRpcMessageKind::Invalid;
    };
    if object.get("jsonrpc").and_then(Value::as_str) != Some("2.0") {
        return JsonRpcMessageKind::Invalid;
    }
    let id = classify_id(object);
    if let Some(result) = object.get("result") {
        return match id {
            JsonRpcIdState::Valid(id) => JsonRpcMessageKind::Response {
                id,
                result: result.clone(),
            },
            JsonRpcIdState::Absent | JsonRpcIdState::Null | JsonRpcIdState::Invalid => {
                JsonRpcMessageKind::Invalid
            }
        };
    }
    if let Some(raw_error) = object.get("error") {
        let Ok(error) = serde_json::from_value(raw_error.clone()) else {
            return JsonRpcMessageKind::Invalid;
        };
        return match id {
            JsonRpcIdState::Valid(id) => JsonRpcMessageKind::Error {
                id: Some(id),
                error,
            },
            JsonRpcIdState::Null => JsonRpcMessageKind::Error { id: None, error },
            JsonRpcIdState::Absent | JsonRpcIdState::Invalid => JsonRpcMessageKind::Invalid,
        };
    }
    let Some(method) = object.get("method").and_then(Value::as_str) else {
        return JsonRpcMessageKind::Invalid;
    };
    match id {
        JsonRpcIdState::Valid(id) => JsonRpcMessageKind::Request {
            id,
            method: method.to_owned(),
        },
        JsonRpcIdState::Absent => JsonRpcMessageKind::Notification {
            method: method.to_owned(),
        },
        JsonRpcIdState::Null | JsonRpcIdState::Invalid => JsonRpcMessageKind::Invalid,
    }
}

fn classify_id(object: &Map<String, Value>) -> JsonRpcIdState {
    match object.get("id") {
        None => JsonRpcIdState::Absent,
        Some(Value::Null) => JsonRpcIdState::Null,
        Some(value) => match serde_json::from_value(value.clone()) {
            Ok(id) => JsonRpcIdState::Valid(id),
            Err(_) => JsonRpcIdState::Invalid,
        },
    }
}
