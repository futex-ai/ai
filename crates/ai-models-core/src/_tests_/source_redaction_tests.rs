//! All modality source chains keep echoed keys and signed URLs private.

use std::error::Error;

use ai_interface::{
    ErrorCore, ErrorKind, ImageGenerationError, JudgmentError, ModelError, TranscriptionError,
    VideoGenerationError,
};
use reqwest::IntoUrl;
use serde_json::json;

use super::error_context_tests::context;

const KEY: &str = "placeholder-echoed-key";
const SIGNATURE: &str = "placeholder-signed-token";

fn modality_error(modality: usize, signed_url: bool) -> Box<dyn Error> {
    let context = context("provider", "model");
    context.add_secret(KEY);
    let source = if signed_url {
        json_http::Error::ReqwestTransport {
            source: invalid_url("://invalid").with_url(
                "https://user:password@provider.test/file?sig=placeholder-signed-token#fragment"
                    .parse()
                    .unwrap(),
            ),
        }
    } else {
        json_http::Error::DeserializeResponse {
            body: json!({"error": KEY}),
            source: serde_json::from_value::<bool>(json!(KEY)).unwrap_err(),
        }
    };
    let core = ErrorCore::new(
        ErrorKind::InvalidOutput,
        "provider",
        "model",
        "safe message",
    );
    match modality {
        0 => Box::new(ModelError::with_source(core, context.source(source))),
        1 => Box::new(ImageGenerationError::with_source(
            core,
            context.source(source),
        )),
        2 => Box::new(VideoGenerationError::with_source(
            core,
            context.source(source),
        )),
        3 => Box::new(TranscriptionError::with_source(
            core,
            context.source(source),
        )),
        4 => Box::new(JudgmentError::with_source(core, context.source(source))),
        _ => unreachable!(),
    }
}

fn invalid_url(url: impl IntoUrl) -> reqwest::Error {
    url.into_url().unwrap_err()
}

fn assert_safe_chain(modality: usize, signed_url: bool) {
    let error = modality_error(modality, signed_url);
    let mut link = Some(error.as_ref());
    let mut links = 0;
    while let Some(error) = link {
        for diagnostic in [error.to_string(), format!("{error:?}")] {
            for secret in [KEY, SIGNATURE, "password", "fragment"] {
                assert!(
                    !diagnostic.contains(secret),
                    "modality {modality}: {diagnostic}"
                );
            }
        }
        links += 1;
        link = error.source();
    }
    assert!(links >= 3, "the typed source chain must remain available");
}

macro_rules! source_cases {
    ($echo:ident, $signed:ident, $modality:expr) => {
        #[test]
        fn $echo() {
            assert_safe_chain($modality, false);
        }

        #[test]
        fn $signed() {
            assert_safe_chain($modality, true);
        }
    };
}

source_cases!(model_echoed_key_is_safe, model_signed_url_is_safe, 0);
source_cases!(image_echoed_key_is_safe, image_signed_url_is_safe, 1);
source_cases!(video_echoed_key_is_safe, video_signed_url_is_safe, 2);
source_cases!(
    transcription_echoed_key_is_safe,
    transcription_signed_url_is_safe,
    3
);
source_cases!(judgment_echoed_key_is_safe, judgment_signed_url_is_safe, 4);
