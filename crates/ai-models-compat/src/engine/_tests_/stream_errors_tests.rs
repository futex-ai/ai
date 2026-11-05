//! Shared error classification over the moved accumulator error contract.

use crate::ChatCompletionsStreamError;
use ai_interface::ModelError;
use ai_models_core::classify_stream_error;

#[test]
fn classifies_eof_and_native_errors_by_provider_event_progress() {
    let eof = ChatCompletionsStreamError::MissingDone;
    let before = classify_stream_error("qwen", "qwen-plus", 0, &eof);
    let native = std::io::Error::other("provider error event");
    let after = classify_stream_error("qwen", "qwen-plus", 2, &native);

    assert!(matches!(before, ModelError::TransientProvider { .. }));
    assert!(matches!(after, ModelError::Interrupted { .. }));
}
