use std::sync::Arc;

use unimock::{MockFn, Unimock, matching};

use crate::{
    InMemoryToolOutputStore, OutputIdGenerationError, ToolOutputIdGeneratorMock, ToolOutputPolicy,
    ToolOutputStore, ToolOutputStoreError, ToolOutputWriteRequest,
};

#[tokio::test]
async fn id_generation_failure_rolls_back_reservation_as_write_failure() {
    let id_generator = Arc::new(Unimock::new(
        ToolOutputIdGeneratorMock
            .next_call(matching!())
            .returns(Err(OutputIdGenerationError::Entropy {
                source: getrandom::Error::UNSUPPORTED,
            })),
    ));
    let store = InMemoryToolOutputStore::with_output_id_generator_for_test(id_generator);
    let policy = ToolOutputPolicy::new(4, 4, 20, 20).unwrap();

    let error = store
        .write(ToolOutputWriteRequest {
            tool_name: "search".to_owned(),
            content: "abcdef".to_owned(),
            policy,
            first_window_length: policy.inline_limit_bytes(),
        })
        .await
        .expect_err("id generation failure should fail the store write");

    assert_eq!(store.reserved_bytes(), 0);
    let ToolOutputStoreError::WriteFailure { source, .. } = error else {
        panic!("id generation should surface as a write failure");
    };
    assert!(matches!(
        source.downcast_ref::<OutputIdGenerationError>(),
        Some(OutputIdGenerationError::Entropy { .. })
    ));
}
