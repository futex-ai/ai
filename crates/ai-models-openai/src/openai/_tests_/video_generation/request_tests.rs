//! OpenAI video request mapping tests.

use ai_interface::{
    VideoGenerationAspect, VideoGenerationDuration, VideoGenerationError,
    VideoGenerationInputImage, VideoGenerationRequest,
};
use serde_json::json;

use super::super::request::{OpenAiVideoApiRequest, build_request};

#[test]
fn maps_text_requests_to_portable_json_controls() {
    let OpenAiVideoApiRequest::Json(body) = build_request(
        "sora-2",
        &VideoGenerationRequest {
            prompt: "A paper bird takes flight".to_owned(),
            aspect: VideoGenerationAspect::Portrait,
            duration: VideoGenerationDuration::Seconds8,
            ..VideoGenerationRequest::default()
        },
    )
    .unwrap() else {
        panic!("text request should use JSON");
    };

    assert_eq!(
        serde_json::to_value(body).unwrap(),
        json!({
            "model": "sora-2",
            "prompt": "A paper bird takes flight",
            "seconds": "8",
            "size": "720x1280"
        })
    );
}

#[test]
fn maps_first_frame_to_multipart_input_reference() {
    let OpenAiVideoApiRequest::Multipart(fields) = build_request(
        "sora-2",
        &VideoGenerationRequest {
            prompt: "Animate this".to_owned(),
            input_image: Some(VideoGenerationInputImage {
                data: vec![1, 2, 3],
                mime_type: "image/webp".to_owned(),
            }),
            ..VideoGenerationRequest::default()
        },
    )
    .unwrap() else {
        panic!("first-frame request should use multipart");
    };

    assert_field(&fields, "model", b"sora-2");
    assert_field(&fields, "seconds", b"4");
    assert_field(&fields, "size", b"1280x720");
    assert!(fields.iter().any(|field| {
        field.name == "input_reference"
            && field.filename.as_deref() == Some("input.webp")
            && field.content_type.as_deref() == Some("image/webp")
            && field.bytes == vec![1, 2, 3]
    }));
}

#[test]
fn rejects_blank_prompts_and_unsupported_first_frames() {
    assert!(matches!(
        build_request("sora-2", &VideoGenerationRequest::default()),
        Err(VideoGenerationError::EmptyPrompt)
    ));
    assert!(matches!(
        build_request(
            "sora-2",
            &VideoGenerationRequest {
                prompt: "Animate".to_owned(),
                input_image: Some(VideoGenerationInputImage {
                    data: vec![1],
                    mime_type: "image/gif".to_owned(),
                }),
                ..VideoGenerationRequest::default()
            }
        ),
        Err(VideoGenerationError::UnsupportedMediaType { content_type })
            if content_type == "image/gif"
    ));
}

fn assert_field(fields: &[json_http::JsonHttpMultipartField], name: &str, bytes: &[u8]) {
    assert!(
        fields
            .iter()
            .any(|field| field.name == name && field.bytes == bytes)
    );
}
