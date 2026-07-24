//! Tool-adapter test fixtures.

use std::sync::Arc;

use serde_json::json;
use unimock::Unimock;

use crate::{DynMcpClient, McpToolDescriptor};

pub(super) fn descriptor(name: &str) -> McpToolDescriptor {
    McpToolDescriptor {
        name: name.to_owned(),
        title: None,
        description: None,
        input_schema: json!({"type":"object"}),
        output_schema: None,
    }
}

pub(super) fn unused_client() -> DynMcpClient {
    Arc::new(Unimock::new(())) as Arc<dyn crate::McpClient>
}

pub(super) fn client(mock: Unimock) -> DynMcpClient {
    Arc::new(mock)
}
