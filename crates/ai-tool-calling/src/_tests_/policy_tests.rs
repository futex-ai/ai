use std::sync::Arc;

use ai_interface::{Model, NoopLogger};
use unimock::Unimock;

use crate::{
    InMemoryToolOutputStore, ToolCallingRuntime, ToolOutputPolicy, ToolOutputPolicyError,
    ToolOutputPolicyLimits,
};

#[tokio::test]
async fn policy_defaults_and_validation_match_protocol_limits() {
    let policy = ToolOutputPolicy::default();

    assert_eq!(policy.inline_limit_bytes(), 20_000);
    assert_eq!(policy.read_limit_bytes(), 20_000);
    assert_eq!(policy.per_output_limit_bytes(), 1_048_576);
    assert_eq!(policy.aggregate_limit_bytes(), 16_777_216);
    assert!(matches!(
        ToolOutputPolicy::new(0, 1, 1, 1),
        Err(ToolOutputPolicyError::ZeroInlineLimit)
    ));
    assert!(matches!(
        ToolOutputPolicy::new(1, 0, 1, 1),
        Err(ToolOutputPolicyError::ZeroReadLimit)
    ));
    assert!(matches!(
        ToolOutputPolicy::new(1, 1, 0, 1),
        Err(ToolOutputPolicyError::ZeroPerOutputLimit)
    ));
    assert!(matches!(
        ToolOutputPolicy::new(1, 1, 1, 0),
        Err(ToolOutputPolicyError::ZeroAggregateLimit)
    ));
    assert!(matches!(
        ToolOutputPolicy::new(2, 1, 1, 1),
        Err(ToolOutputPolicyError::InlineLimitExceedsPerOutput { .. })
    ));
    assert!(matches!(
        ToolOutputPolicy::new(1, 2, 1, 1),
        Err(ToolOutputPolicyError::ReadLimitExceedsPerOutput { .. })
    ));
    assert!(matches!(
        ToolOutputPolicy::new(1, 1, 2, 1),
        Err(ToolOutputPolicyError::PerOutputLimitExceedsAggregate { .. })
    ));
}

#[test]
fn raw_limit_constructor_fails_runtime_construction_for_invalid_policy() {
    let model: Arc<dyn Model> = Arc::new(Unimock::new(()));
    let error = match ToolCallingRuntime::new_with_output_policy_limits(
        "system prompt",
        model,
        Arc::new(NoopLogger),
        Vec::new(),
        Arc::new(InMemoryToolOutputStore::new()),
        ToolOutputPolicyLimits {
            inline_limit_bytes: 0,
            read_limit_bytes: 1,
            per_output_limit_bytes: 1,
            aggregate_limit_bytes: 1,
        },
    ) {
        Ok(_) => panic!("invalid policy should fail runtime construction"),
        Err(error) => error,
    };

    assert!(matches!(
        error,
        crate::Error::OutputPolicy {
            source: ToolOutputPolicyError::ZeroInlineLimit
        }
    ));
}
