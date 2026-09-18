//! TypeSafe judgment request mapping.

use std::collections::BTreeMap;

use ai_interface::{JudgmentContent, JudgmentQuestion, JudgmentRequest};
use serde::Serialize;

#[derive(Debug, Serialize)]
/// Serialized System One request body.
pub(super) struct TypeSafeRequest<'a> {
    state: &'a JudgmentContent,
    model: &'a str,
    questions: BTreeMap<&'a str, TypeSafeQuestion<'a>>,
}

#[derive(Debug, Serialize)]
#[serde(tag = "type")]
enum TypeSafeQuestion<'a> {
    #[serde(rename = "noul")]
    Condition {
        instructions: &'a JudgmentContent,
        #[serde(skip_serializing_if = "Option::is_none")]
        criteria: Option<TypeSafeConditionCriteria<'a>>,
    },
    #[serde(rename = "choice")]
    Choice {
        instructions: &'a JudgmentContent,
        criteria: &'a BTreeMap<String, Option<JudgmentContent>>,
    },
    #[serde(rename = "score")]
    Score {
        instructions: &'a JudgmentContent,
        criteria: &'a [Option<JudgmentContent>],
    },
}

#[derive(Debug, Serialize)]
struct TypeSafeConditionCriteria<'a> {
    #[serde(rename = "true", skip_serializing_if = "Option::is_none")]
    yes: Option<&'a JudgmentContent>,
    #[serde(rename = "false", skip_serializing_if = "Option::is_none")]
    no: Option<&'a JudgmentContent>,
}

/// Maps a shared judgment request to the System One wire contract.
pub(super) fn build_request<'a>(
    model_id: &'a str,
    request: &'a JudgmentRequest,
) -> TypeSafeRequest<'a> {
    TypeSafeRequest {
        state: &request.state,
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
            criteria: options,
        },
        JudgmentQuestion::Score {
            instructions,
            levels,
        } => TypeSafeQuestion::Score {
            instructions,
            criteria: levels,
        },
    }
}

#[cfg(test)]
#[path = "_tests_/request_tests.rs"]
mod request_tests;
