//! TypeSafe support checks before HTTP dispatch.

use std::{collections::BTreeMap, sync::Arc};

use ai_interface::{
    ConversationMessage, ConversationRole, ErrorKind, JudgmentContent, JudgmentError,
    JudgmentInput, JudgmentInputFeature, JudgmentInputForm, JudgmentModel, JudgmentPart,
    JudgmentPartKind, JudgmentQuestion, JudgmentResponse, JudgmentResult,
};
use json_http::{
    MemoryRecordingLog, RecordingLog, RecordingTransport, ReplayTransport,
    TransportBackedJsonHttpClient,
};

use super::{TypeSafeJudgmentModel, test_support::simple_request};

#[tokio::test]
async fn rejects_image_parts_without_transport() {
    let content = JudgmentContent {
        parts: vec![JudgmentPart::Image {
            mime_type: "image/png".to_owned(),
            data_base64: "AQID".to_owned(),
        }],
    };
    let mut image_input = simple_request();
    image_input.input = content.clone().into();
    let mut image_instructions = simple_request();
    image_instructions.questions = BTreeMap::from([(
        "is_urgent".to_owned(),
        JudgmentQuestion::condition(content, None),
    )]);

    let (model, requests) = model_with_transport_log();
    for (case, request) in [("input", image_input), ("instructions", image_instructions)] {
        let result = model.judge(&request).await;

        assert_eq!(requests.snapshot().len(), 0, "{case}");
        assert_unsupported(
            result,
            JudgmentInputFeature::ContentPart(JudgmentPartKind::Image),
        );
    }
}

#[tokio::test]
async fn rejects_conversations_without_transport() {
    let mut request = simple_request();
    request.input = JudgmentInput::Conversation {
        system: None,
        messages: vec![ConversationMessage::user("My invoice is due.")],
    };
    let (model, requests) = model_with_transport_log();

    let result = model.judge(&request).await;

    assert_eq!(requests.snapshot().len(), 0);
    assert_unsupported(
        result,
        JudgmentInputFeature::InputForm(JudgmentInputForm::Conversation),
    );

    request.input = JudgmentInput::Conversation {
        system: None,
        messages: vec![ConversationMessage::new(ConversationRole::Tool, vec![])],
    };
    let invalid = model.judge(&request).await;

    assert_eq!(requests.snapshot().len(), 0);
    assert!(
        matches!(&invalid, Err(error) if error.core().kind == ErrorKind::InvalidRequest),
        "{invalid:?}"
    );
}

fn model_with_transport_log() -> (TypeSafeJudgmentModel, Arc<MemoryRecordingLog>) {
    let log = Arc::new(MemoryRecordingLog::default());
    let transport =
        RecordingTransport::new(Arc::new(ReplayTransport::new(Vec::new())), log.clone());
    let http = Arc::new(TransportBackedJsonHttpClient::new(Arc::new(transport)));
    (
        TypeSafeJudgmentModel::new(http, "jev-latest", "test-key"),
        log,
    )
}

fn assert_unsupported(
    result: JudgmentResult<JudgmentResponse>,
    expected_feature: JudgmentInputFeature,
) {
    assert!(
        matches!(&result, Err(JudgmentError::UnsupportedInput { core, feature })
            if core.kind == ErrorKind::UnsupportedInput && core.provider == "typesafe"
            && core.model_id == "jev-latest" && *feature == expected_feature),
        "{result:?}"
    );
}
