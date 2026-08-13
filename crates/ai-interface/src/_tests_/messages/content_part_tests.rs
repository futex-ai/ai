//! Conversation content-part serde tests.

use serde_json::json;

use crate::ConversationContentPart;

#[test]
fn video_part_serializes_with_stable_wire_shape() {
    let part = ConversationContentPart::Video {
        mime_type: "video/mp4".to_owned(),
        data_base64: "dmlkZW8=".to_owned(),
    };

    assert_eq!(
        serde_json::to_value(&part).expect("video part should serialize"),
        json!({
            "type": "video",
            "mime_type": "video/mp4",
            "data_base64": "dmlkZW8="
        })
    );
}

#[test]
fn video_part_deserializes_from_wire_shape() {
    let part: ConversationContentPart = serde_json::from_value(json!({
        "type": "video",
        "mime_type": "video/webm",
        "data_base64": "d2VibQ=="
    }))
    .expect("video part should deserialize");

    assert_eq!(
        part,
        ConversationContentPart::Video {
            mime_type: "video/webm".to_owned(),
            data_base64: "d2VibQ==".to_owned(),
        }
    );
}

#[test]
fn ordered_text_image_and_video_parts_round_trip() {
    let parts = vec![
        ConversationContentPart::Text {
            text: "before".to_owned(),
        },
        ConversationContentPart::Image {
            mime_type: "image/png".to_owned(),
            data_base64: "aW1n".to_owned(),
        },
        ConversationContentPart::Video {
            mime_type: "video/mp4".to_owned(),
            data_base64: "dmlk".to_owned(),
        },
    ];

    let value = serde_json::to_value(&parts).expect("parts should serialize");
    let decoded: Vec<ConversationContentPart> =
        serde_json::from_value(value).expect("parts should deserialize");

    assert_eq!(decoded, parts);
}
