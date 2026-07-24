//! JSON-RPC message construction and classification.

use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

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
    let id = object
        .get("id")
        .and_then(|value| serde_json::from_value(value.clone()).ok());
    if let Some(result) = object.get("result") {
        return id.map_or(JsonRpcMessageKind::Invalid, |id| {
            JsonRpcMessageKind::Response {
                id,
                result: result.clone(),
            }
        });
    }
    if let Some(raw_error) = object.get("error") {
        return match serde_json::from_value(raw_error.clone()) {
            Ok(error) => JsonRpcMessageKind::Error { id, error },
            Err(_) => JsonRpcMessageKind::Invalid,
        };
    }
    let Some(method) = object.get("method").and_then(Value::as_str) else {
        return JsonRpcMessageKind::Invalid;
    };
    if let Some(id) = id {
        JsonRpcMessageKind::Request {
            id,
            method: method.to_owned(),
        }
    } else {
        JsonRpcMessageKind::Notification {
            method: method.to_owned(),
        }
    }
}
