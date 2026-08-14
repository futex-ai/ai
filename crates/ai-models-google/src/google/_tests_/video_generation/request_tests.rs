//! Google video request mapping tests.

use ai_interface::{
    VideoGenerationAspect, VideoGenerationDuration, VideoGenerationError,
    VideoGenerationInputImage, VideoGenerationRequest,
};
use serde_json::json;

use super::super::request::build_request;

#[test]
fn maps_text_request_to_one_portable_video() {
    let body = build_request(&VideoGenerationRequest {
        prompt: "A paper bird takes flight".to_owned(),
        aspect: VideoGenerationAspect::Portrait,
        duration: VideoGenerationDuration::Seconds8,
        ..VideoGenerationRequest::default()
    })
    .unwrap();

    assert_eq!(
        serde_json::to_value(body).unwrap(),
        json!({
            "instances": [{"prompt": "A paper bird takes flight"}],
            "parameters": {
                "aspectRatio": "9:16",
                "durationSeconds": 8,
                "resolution": "720p",
                "sampleCount": 1
            }
        })
    );
}

#[test]
fn maps_first_frame_to_inline_data() {
    let body = build_request(&VideoGenerationRequest {
        prompt: "Animate this".to_owned(),
        input_image: Some(VideoGenerationInputImage {
            data: vec![1, 2, 3],
            mime_type: "image/png".to_owned(),
        }),
        ..VideoGenerationRequest::default()
    })
    .unwrap();
    let value = serde_json::to_value(body).unwrap();

    assert_eq!(
        value["instances"][0]["image"]["inlineData"]["mimeType"],
        "image/png"
    );
    assert_eq!(value["instances"][0]["image"]["inlineData"]["data"], "AQID");
    assert_eq!(value["parameters"]["aspectRatio"], "16:9");
    assert_eq!(value["parameters"]["durationSeconds"], 4);
}

#[test]
fn rejects_blank_prompts_and_unsupported_first_frames() {
    assert!(matches!(
        build_request(&VideoGenerationRequest::default()),
        Err(VideoGenerationError::EmptyPrompt)
    ));
    assert!(matches!(
        build_request(&VideoGenerationRequest {
            prompt: "Animate".to_owned(),
            input_image: Some(VideoGenerationInputImage {
                data: vec![1],
                mime_type: "image/gif".to_owned(),
            }),
            ..VideoGenerationRequest::default()
        }),
        Err(VideoGenerationError::UnsupportedMediaType { content_type })
            if content_type == "image/gif"
    ));
}
