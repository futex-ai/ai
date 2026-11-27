//! TypeSafe judgment request mapping.

use std::collections::BTreeMap;

use ai_interface::{
    JudgmentContent, JudgmentDescription, JudgmentInput, JudgmentPart, JudgmentQuestion,
    JudgmentRequest,
};
use json_http::OrderedObject;
use serde::{Serialize, Serializer, ser::Error};

#[derive(Debug, Serialize)]
/// Serialized System One request body.
pub(super) struct TypeSafeRequest<'a> {
    #[serde(serialize_with = "serialize_input")]
    state: &'a JudgmentInput,
    model: &'a str,
    questions: BTreeMap<&'a str, TypeSafeQuestion<'a>>,
}

#[derive(Debug, Serialize)]
#[serde(tag = "type")]
enum TypeSafeQuestion<'a> {
    #[serde(rename = "noul")]
    Condition {
        #[serde(serialize_with = "serialize_content")]
        instructions: &'a JudgmentContent,
        #[serde(skip_serializing_if = "Option::is_none")]
        criteria: Option<TypeSafeConditionCriteria<'a>>,
    },
    #[serde(rename = "choice")]
    Choice {
        #[serde(serialize_with = "serialize_content")]
        instructions: &'a JudgmentContent,
        criteria: OrderedObject<&'a str, Option<&'a JudgmentDescription>>,
    },
    #[serde(rename = "score")]
    Score {
        #[serde(serialize_with = "serialize_content")]
        instructions: &'a JudgmentContent,
        criteria: Vec<TypeSafeLevel<'a>>,
    },
}

#[derive(Debug, Serialize)]
struct TypeSafeConditionCriteria<'a> {
    #[serde(rename = "true", skip_serializing_if = "Option::is_none")]
    yes: Option<&'a JudgmentDescription>,
    #[serde(rename = "false", skip_serializing_if = "Option::is_none")]
    no: Option<&'a JudgmentDescription>,
}

#[derive(Debug, Serialize)]
#[serde(untagged)]
enum TypeSafeLevel<'a> {
    Label(&'a str),
    Described {
        label: &'a str,
        description: &'a JudgmentDescription,
    },
}

/// Maps a shared judgment request to the System One wire contract.
pub(super) fn build_request<'a>(
    model_id: &'a str,
    request: &'a JudgmentRequest,
) -> TypeSafeRequest<'a> {
    TypeSafeRequest {
        state: &request.input,
        model: model_id,
        questions: request
            .questions
            .iter()
            .map(|(id, question)| (id.as_str(), map_question(question)))
            .collect(),
    }
}

fn map_question(question: &JudgmentQuestion) -> TypeSafeQuestion<'_> {
    match question {
        JudgmentQuestion::Condition {
            instructions,
            criteria,
        } => TypeSafeQuestion::Condition {
            instructions,
            criteria: criteria.as_ref().map(|criteria| TypeSafeConditionCriteria {
                yes: criteria.yes.as_ref(),
                no: criteria.no.as_ref(),
            }),
        },
        JudgmentQuestion::Choice {
            instructions,
            options,
        } => TypeSafeQuestion::Choice {
            instructions,
            criteria: OrderedObject::new(
                options
                    .iter()
                    .map(|option| (option.label.as_str(), option.description.as_ref())),
            ),
        },
        JudgmentQuestion::Score {
            instructions,
            levels,
        } => TypeSafeQuestion::Score {
            instructions,
            criteria: levels
                .iter()
                .map(|level| match &level.description {
                    Some(description) => TypeSafeLevel::Described {
                        label: &level.label,
                        description,
                    },
                    None => TypeSafeLevel::Label(&level.label),
                })
                .collect(),
        },
    }
}

fn serialize_input<S: Serializer>(input: &JudgmentInput, serializer: S) -> Result<S::Ok, S::Error> {
    match input {
        JudgmentInput::Content(content) => serialize_content(content, serializer),
        JudgmentInput::Conversation { .. } => Err(S::Error::custom(
            "unexpected conversation input in TypeSafe request",
        )),
    }
}

fn serialize_content<S: Serializer>(
    content: &JudgmentContent,
    serializer: S,
) -> Result<S::Ok, S::Error> {
    match content.parts.as_slice() {
        [part] => TypeSafePart(part).serialize(serializer),
        parts => serializer.collect_seq(parts.iter().map(TypeSafePart)),
    }
}

struct TypeSafePart<'a>(&'a JudgmentPart);

impl Serialize for TypeSafePart<'_> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        match self.0 {
            JudgmentPart::Text { text } => text.serialize(serializer),
            JudgmentPart::Json { value } => value.serialize(serializer),
            JudgmentPart::Image { .. } => {
                Err(S::Error::custom("unexpected image in TypeSafe request"))
            }
        }
    }
}

#[cfg(test)]
#[path = "_tests_/request_tests.rs"]
mod request_tests;
