//! Provider-neutral judgment support checks.

use std::collections::BTreeMap;

use crate::{
    ConversationMessage, ConversationPart, ErrorKind, JudgmentContent,
    JudgmentConversationPartKind, JudgmentError, JudgmentInput, JudgmentInputFeature,
    JudgmentInputForm, JudgmentPart, JudgmentPartKind, JudgmentQuestion, JudgmentRequest,
    JudgmentSupport,
};

const SUPPORT: JudgmentSupport = JudgmentSupport {
    content_parts: &[JudgmentPartKind::Text, JudgmentPartKind::Json],
    input_forms: &[JudgmentInputForm::Content, JudgmentInputForm::Conversation],
    conversation_parts: &[JudgmentConversationPartKind::Text],
};

#[test]
fn support_checks_input_content_and_every_question_kind() {
    let cases = [
        (
            "input",
            request(
                image_content().into(),
                JudgmentQuestion::condition("condition", None),
            ),
        ),
        (
            "condition",
            request(
                "ticket".into(),
                JudgmentQuestion::condition(image_content(), None),
            ),
        ),
        (
            "choice",
            request(
                "ticket".into(),
                JudgmentQuestion::choice(image_content(), [("yes", None)]),
            ),
        ),
        (
            "score",
            request(
                "ticket".into(),
                JudgmentQuestion::score(image_content(), ["low", "high"]),
            ),
        ),
    ];

    for (case, request) in cases {
        assert_unsupported(
            SUPPORT,
            &request,
            JudgmentInputFeature::ContentPart(JudgmentPartKind::Image),
            case,
        );
        JudgmentSupport::ALL
            .check(&request, "fixture", "model")
            .unwrap();
    }
}

#[test]
fn support_checks_the_conversation_form_and_later_message_parts() {
    let input = JudgmentInput::Conversation {
        system: None,
        messages: vec![
            ConversationMessage::user("Read the ticket."),
            ConversationMessage::user_with_parts(vec![
                ConversationPart::text("See the attachment."),
                ConversationPart::video("video/mp4", "AQID"),
            ]),
        ],
    };
    let request = request(input, JudgmentQuestion::condition("Is it urgent?", None));

    assert_unsupported(
        JudgmentSupport {
            input_forms: &[JudgmentInputForm::Content],
            ..SUPPORT
        },
        &request,
        JudgmentInputFeature::InputForm(JudgmentInputForm::Conversation),
        "conversation form",
    );
    assert_unsupported(
        SUPPORT,
        &request,
        JudgmentInputFeature::ConversationPart(JudgmentConversationPartKind::Video),
        "video part",
    );
    JudgmentSupport::ALL
        .check(&request, "fixture", "model")
        .unwrap();
}

fn request(input: JudgmentInput, question: JudgmentQuestion) -> JudgmentRequest {
    JudgmentRequest {
        input,
        questions: BTreeMap::from([
            (
                "first".to_owned(),
                JudgmentQuestion::condition("text only", None),
            ),
            ("second".to_owned(), question),
        ]),
    }
}

fn image_content() -> JudgmentContent {
    JudgmentContent {
        parts: vec![
            JudgmentPart::Text {
                text: "Read the attachment.".to_owned(),
            },
            JudgmentPart::Image {
                mime_type: "image/png".to_owned(),
                data_base64: "AQID".to_owned(),
            },
        ],
    }
}

fn assert_unsupported(
    support: JudgmentSupport,
    request: &JudgmentRequest,
    expected_feature: JudgmentInputFeature,
    case: &str,
) {
    request.validate().unwrap();
    let result = support.check(request, "fixture", "model");
    assert!(
        matches!(&result, Err(JudgmentError::UnsupportedInput { core, feature })
            if core.kind == ErrorKind::UnsupportedInput && core.provider == "fixture"
            && core.model_id == "model" && *feature == expected_feature),
        "{case}: {result:?}"
    );
}
