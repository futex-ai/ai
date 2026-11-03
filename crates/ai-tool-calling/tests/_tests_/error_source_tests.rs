//! Loop-created errors retain private causes only through the safe carrier.

use std::error::Error;

use ai_interface::{LoggerError, SafeErrorSource, Tool, ToolError, ToolResult};
use ai_tool_calling::{TypedTool, TypedToolAdapter};
use async_trait::async_trait;
use serde::{Serialize, Serializer};
use serde_json::json;

use super::typed_tool_tests::Arguments;

#[derive(Debug, thiserror::Error)]
#[error("private-cause-content")]
struct PrivateCause;

fn check_cause(error: &dyn Error) {
    let diagnostic = format!("{error}\n{error:?}");
    assert!(!diagnostic.contains("private-cause-content"));
    let cause = error.source().unwrap();
    assert!(!format!("{cause}\n{cause:?}").contains("private-cause-content"));
    let safe = cause.downcast_ref::<SafeErrorSource>();
    assert!(safe.is_some(), "private cause must use the shared carrier");
    assert!(safe.unwrap().raw_cause().is::<PrivateCause>());
}

#[test]
fn checkpoint_and_observer_errors_keep_private_causes_in_the_carrier() {
    check_cause(&ai_tool_calling::Error::checkpoint(PrivateCause));
    check_cause(&LoggerError::write(PrivateCause));
}

struct FailingOutput;

impl Serialize for FailingOutput {
    fn serialize<S: Serializer>(&self, _: S) -> Result<S::Ok, S::Error> {
        Err(serde::ser::Error::custom("private-cause-content"))
    }
}

struct FailingSerialization;

#[async_trait]
impl TypedTool for FailingSerialization {
    type Args = Arguments;
    type Output = FailingOutput;

    fn name(&self) -> &str {
        "serialize"
    }
    fn description(&self) -> &str {
        "Return a serialization failure."
    }
    async fn execute(&self, _: Arguments, _: &str) -> ToolResult<FailingOutput> {
        Ok(FailingOutput)
    }
}

#[tokio::test]
async fn typed_output_serialization_errors_use_the_safe_execution_error() {
    let tool = TypedToolAdapter::new(FailingSerialization);
    let result = tool
        .call("serialize", json!({"message":"hi","repeat":1}))
        .await;
    assert!(
        matches!(result, Err(ToolError::Execution { .. })),
        "serialization must fail at the tool boundary: {result:?}"
    );
    let error = result.unwrap_err();
    assert!(!format!("{error}\n{error:?}").contains("private-cause-content"));
    let ToolError::Execution { source, .. } = error else {
        unreachable!()
    };
    let safe = source.downcast_ref::<SafeErrorSource>();
    assert!(safe.is_some());
    assert!(safe.unwrap().raw_cause().is::<serde_json::Error>());
}
