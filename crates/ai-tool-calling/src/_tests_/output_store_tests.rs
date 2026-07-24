use ai_interface::ToolOutputId;

use crate::{
    InMemoryToolOutputStore, ToolOutputPolicy, ToolOutputStore, ToolOutputStoreError,
    ToolOutputStoreReadRequest, ToolOutputWriteRequest,
};

#[tokio::test]
async fn successful_write_generates_uuid_v7_id_and_reads_windows() {
    let store = InMemoryToolOutputStore::new();
    let policy = ToolOutputPolicy::new(4, 4, 20, 20).unwrap();

    let written = store
        .write(write_request("search", "abcdef", policy))
        .await
        .unwrap();
    let id = written.output_id.as_str();
    assert!(id.starts_with("toolout_"));
    assert_eq!(id.len(), "toolout_".len() + 36);
    assert_eq!(id.as_bytes()["toolout_".len() + 14], b'7');
    assert!(id.chars().all(|char| {
        char.is_ascii_lowercase() || char.is_ascii_digit() || char == '-' || char == '_'
    }));
    assert_eq!(written.first_window.content, "abcd");
    assert_eq!(written.first_window.next_offset, Some(4));

    let read = store
        .read(read_request(written.output_id, Some(4), Some(4), policy))
        .await
        .unwrap();

    assert_eq!(read.tool_name, "search");
    assert_eq!(read.content, "ef");
    assert!(!read.truncated);
}

#[tokio::test]
async fn unavailable_ids_and_scope_isolation_share_one_error() {
    let store = InMemoryToolOutputStore::new();
    let other_store = InMemoryToolOutputStore::new();
    let policy = ToolOutputPolicy::new(4, 4, 20, 20).unwrap();
    let written = store
        .write(write_request("search", "abcdef", policy))
        .await
        .unwrap();

    let missing = other_store
        .read(read_request(written.output_id, None, None, policy))
        .await
        .expect_err("other store must not resolve id");

    assert!(matches!(
        missing,
        ToolOutputStoreError::UnavailableOutput { .. }
    ));
}

#[tokio::test]
async fn reads_snap_end_backward_to_utf8_boundary() {
    let store = InMemoryToolOutputStore::new();
    let policy = ToolOutputPolicy::new(3, 4, 20, 20).unwrap();
    let written = store
        .write(write_request("search", "aé😀z", policy))
        .await
        .unwrap();

    let first = store
        .read(read_request(
            written.output_id.clone(),
            Some(0),
            Some(3),
            policy,
        ))
        .await
        .unwrap();
    let second = store
        .read(read_request(
            written.output_id,
            first.next_offset,
            Some(4),
            policy,
        ))
        .await
        .unwrap();

    assert_eq!(first.content, "aé");
    assert_eq!(first.returned_bytes, 3);
    assert_eq!(second.content, "😀");
    assert_eq!(second.next_offset, Some(7));
}

#[tokio::test]
async fn end_of_output_read_returns_empty_complete_window() {
    let store = InMemoryToolOutputStore::new();
    let policy = ToolOutputPolicy::new(5, 5, 20, 20).unwrap();
    let written = store
        .write(write_request("search", "abc", policy))
        .await
        .unwrap();

    let read = store
        .read(read_request(written.output_id, Some(3), None, policy))
        .await
        .unwrap();

    assert_eq!(read.content, "");
    assert_eq!(read.returned_bytes, 0);
    assert!(!read.truncated);
}

#[tokio::test]
async fn invalid_offsets_lengths_and_too_short_windows_are_typed() {
    let store = InMemoryToolOutputStore::new();
    let policy = ToolOutputPolicy::new(1, 1, 20, 20).unwrap();
    let written = store
        .write(write_request("search", "aé", policy))
        .await
        .unwrap();

    let bad_offset = store
        .read(read_request(
            written.output_id.clone(),
            Some(2),
            Some(1),
            policy,
        ))
        .await
        .expect_err("offset inside multibyte character should fail");
    let bad_length = store
        .read(read_request(
            written.output_id.clone(),
            Some(0),
            Some(0),
            policy,
        ))
        .await
        .expect_err("zero length should fail");
    let too_short = store
        .read(read_request(written.output_id, Some(1), Some(1), policy))
        .await
        .expect_err("one byte cannot fit e acute");

    assert!(matches!(
        bad_offset,
        ToolOutputStoreError::InvalidOffset { .. }
    ));
    assert!(matches!(
        bad_length,
        ToolOutputStoreError::InvalidLength { .. }
    ));
    assert!(matches!(
        too_short,
        ToolOutputStoreError::NoCompleteCharacterFits {
            minimum_usable_length: 2,
            ..
        }
    ));
}

#[tokio::test]
async fn failed_write_rolls_back_aggregate_reservation() {
    let store = InMemoryToolOutputStore::new();
    let policy = ToolOutputPolicy::new(4, 4, 10, 10).unwrap();
    store.fail_next_write_for_test();

    let error = store
        .write(write_request("search", "abcdef", policy))
        .await
        .expect_err("injected write failure should fail");
    assert!(matches!(error, ToolOutputStoreError::WriteFailure { .. }));
    assert_eq!(store.reserved_bytes(), 0);

    let written = store
        .write(write_request("search", "abcdefghij", policy))
        .await
        .unwrap();

    assert_eq!(store.reserved_bytes(), 10);
    assert_eq!(written.first_window.content, "abcd");
}

#[tokio::test]
async fn aggregate_rejection_is_atomic() {
    let store = InMemoryToolOutputStore::new();
    let policy = ToolOutputPolicy::new(4, 4, 10, 12).unwrap();
    store
        .write(write_request("search", "abcdefgh", policy))
        .await
        .unwrap();

    let error = store
        .write(write_request("search", "abcdef", policy))
        .await
        .expect_err("second write should exceed remaining budget");

    assert!(matches!(
        error,
        ToolOutputStoreError::AggregateExhausted {
            requested_bytes: 6,
            ..
        }
    ));
    assert_eq!(store.reserved_bytes(), 8);
}

#[tokio::test]
async fn per_output_overflow_returns_degraded_preview_without_reserving() {
    let store = InMemoryToolOutputStore::new();
    let policy = ToolOutputPolicy::new(4, 4, 5, 10).unwrap();

    let error = store
        .write(write_request("search", "abcdef", policy))
        .await
        .expect_err("write should exceed per-output limit");

    assert!(matches!(
        error,
        ToolOutputStoreError::PerOutputOverflow {
            requested_bytes: 6,
            ..
        }
    ));
    assert_eq!(store.reserved_bytes(), 0);
}

#[tokio::test]
async fn explicit_length_is_capped_by_policy_read_limit() {
    let store = InMemoryToolOutputStore::new();
    let policy = ToolOutputPolicy::new(4, 4, 20, 20).unwrap();
    let written = store
        .write(write_request("search", "abcdefghij", policy))
        .await
        .unwrap();

    let read = store
        .read(read_request(written.output_id, Some(0), Some(9), policy))
        .await
        .unwrap();

    assert_eq!(read.content, "abcd");
    assert_eq!(read.returned_bytes, 4);
    assert_eq!(read.next_offset, Some(4));
}

#[tokio::test]
async fn omitted_read_arguments_default_to_zero_and_policy_read_limit() {
    let store = InMemoryToolOutputStore::new();
    let policy = ToolOutputPolicy::new(3, 5, 20, 20).unwrap();
    let written = store
        .write(write_request("search", "abcdefghij", policy))
        .await
        .unwrap();

    let read = store
        .read(read_request(written.output_id, None, None, policy))
        .await
        .unwrap();

    assert_eq!(read.offset, 0);
    assert_eq!(read.content, "abcde");
    assert_eq!(read.returned_bytes, 5);
}

fn write_request(
    tool_name: &str,
    content: &str,
    policy: ToolOutputPolicy,
) -> ToolOutputWriteRequest {
    ToolOutputWriteRequest {
        tool_name: tool_name.to_owned(),
        content: content.to_owned(),
        policy,
        first_window_length: policy.inline_limit_bytes(),
    }
}

fn read_request(
    output_id: ToolOutputId,
    offset: Option<usize>,
    length: Option<usize>,
    policy: ToolOutputPolicy,
) -> ToolOutputStoreReadRequest {
    ToolOutputStoreReadRequest {
        output_id,
        offset,
        length,
        policy,
    }
}
