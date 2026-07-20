use crate::{
    ToolOutputEnvelope, ToolOutputId, ToolOutputInlineEnvelope, ToolOutputReadRequest,
    ToolOutputRemainderUnavailableReason, ToolOutputWindowEnvelope,
};
use serde_json::json;

#[test]
fn output_id_serializes_as_an_opaque_string() {
    let id = ToolOutputId::from_opaque("toolout_018f0000-0000-7000-8000-000000000000");

    let value = serde_json::to_value(&id).unwrap();
    let decoded: ToolOutputId = serde_json::from_value(value.clone()).unwrap();

    assert_eq!(value, json!("toolout_018f0000-0000-7000-8000-000000000000"));
    assert_eq!(decoded, id);
}

#[test]
fn inline_envelope_round_trips_without_output_id() {
    let envelope = ToolOutputEnvelope::inline("memory_read", json!({ "entries": [] }), 14);

    let value = serde_json::to_value(&envelope).unwrap();
    let decoded: ToolOutputEnvelope = serde_json::from_value(value.clone()).unwrap();

    assert_eq!(
        value,
        json!({
            "type": "tool_output",
            "tool_name": "memory_read",
            "output": { "entries": [] },
            "total_bytes": 14,
            "truncated": false
        })
    );
    assert_eq!(decoded, envelope);
}

#[test]
fn readable_window_envelope_round_trips_with_next_offset() {
    let id = ToolOutputId::from_opaque("toolout_018f0000-0000-7000-8000-000000000001");
    let envelope = ToolOutputWindowEnvelope::readable(
        "search",
        id.clone(),
        0,
        "{\"items\":[",
        10,
        42,
        Some(10),
    )
    .unwrap();

    let value = serde_json::to_value(&envelope).unwrap();
    let decoded: ToolOutputWindowEnvelope = serde_json::from_value(value.clone()).unwrap();

    assert_eq!(
        value,
        json!({
            "type": "tool_output_window",
            "output_id": id,
            "tool_name": "search",
            "offset": 0,
            "content": "{\"items\":[",
            "returned_bytes": 10,
            "total_bytes": 42,
            "truncated": true,
            "next_offset": 10
        })
    );
    assert_eq!(decoded, envelope);
}

#[test]
fn complete_window_omits_next_offset_and_remainder() {
    let id = ToolOutputId::from_opaque("toolout_018f0000-0000-7000-8000-000000000002");
    let envelope =
        ToolOutputWindowEnvelope::readable("search", id, 20, "done", 4, 24, None).unwrap();

    let value = serde_json::to_value(&envelope).unwrap();
    let decoded: ToolOutputWindowEnvelope = serde_json::from_value(value.clone()).unwrap();

    assert_eq!(value.get("next_offset"), None);
    assert_eq!(value.get("remainder_unavailable"), None);
    assert_eq!(decoded, envelope);
}

#[test]
fn degraded_window_envelopes_round_trip_for_every_reason() {
    for (reason, expected) in [
        (
            ToolOutputRemainderUnavailableReason::OutputTooLarge,
            "output_too_large",
        ),
        (
            ToolOutputRemainderUnavailableReason::BudgetExhausted,
            "budget_exhausted",
        ),
        (
            ToolOutputRemainderUnavailableReason::StoreUnavailable,
            "store_unavailable",
        ),
    ] {
        let envelope =
            ToolOutputEnvelope::degraded_window("search", "{\"items\":[", 10, 42, reason);
        let value = serde_json::to_value(&envelope).unwrap();
        let decoded: ToolOutputEnvelope = serde_json::from_value(value.clone()).unwrap();

        assert_eq!(value.get("output_id"), None);
        assert_eq!(value.get("next_offset"), None);
        assert_eq!(value["remainder_unavailable"], json!(expected));
        assert_eq!(decoded, envelope);
    }
}

#[test]
fn read_request_round_trips_with_optional_fields() {
    let request = ToolOutputReadRequest {
        output_id: ToolOutputId::from_opaque("toolout_018f0000-0000-7000-8000-000000000003"),
        offset: Some(20),
        length: None,
    };

    let value = serde_json::to_value(&request).unwrap();
    let decoded: ToolOutputReadRequest = serde_json::from_value(value.clone()).unwrap();

    assert_eq!(value.get("length"), None);
    assert_eq!(decoded, request);
}

#[test]
fn invalid_window_shapes_are_rejected() {
    let id = "toolout_018f0000-0000-7000-8000-000000000004";

    assert_window_rejected(window_shape(
        Some(id),
        true,
        Some(3),
        Some("budget_exhausted"),
    ));
    assert_window_rejected(window_shape(Some(id), true, None, None));
    assert_window_rejected(window_shape(
        Some(id),
        true,
        None,
        Some("store_unavailable"),
    ));
    assert_window_rejected(window_shape(Some(id), false, Some(3), None));
    assert_window_rejected(window_shape(None, false, None, None));
}

fn assert_window_rejected(value: serde_json::Value) {
    let result = serde_json::from_value::<ToolOutputWindowEnvelope>(value);
    assert!(result.is_err());
}

fn window_shape(
    output_id: Option<&str>,
    truncated: bool,
    next_offset: Option<usize>,
    remainder_unavailable: Option<&str>,
) -> serde_json::Value {
    let mut value = json!({
        "type": "tool_output_window",
        "tool_name": "search",
        "offset": 0,
        "content": "abc",
        "returned_bytes": 3,
        "total_bytes": 10,
        "truncated": truncated
    });
    let object = value.as_object_mut().unwrap();
    if let Some(output_id) = output_id {
        object.insert("output_id".to_owned(), json!(output_id));
    }
    if let Some(next_offset) = next_offset {
        object.insert("next_offset".to_owned(), json!(next_offset));
    }
    if let Some(remainder_unavailable) = remainder_unavailable {
        object.insert(
            "remainder_unavailable".to_owned(),
            json!(remainder_unavailable),
        );
    }
    value
}

#[test]
fn inline_truncated_shape_is_rejected() {
    let value = json!({
        "type": "tool_output",
        "tool_name": "memory_read",
        "output": {},
        "total_bytes": 2,
        "truncated": true
    });

    let result = serde_json::from_value::<ToolOutputInlineEnvelope>(value);

    assert!(result.is_err());
}
