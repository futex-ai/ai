//! Decisions request body and stable question names.

use std::borrow::Cow;

use ai_interface::{JudgmentDescription, JudgmentQuestion, JudgmentRequest};
use serde::Serialize;
use serde_json::Result;

use super::input::{InputItem, InputPart, TextOrArray, input, instructions};

#[derive(Serialize)]
pub(super) struct DecisionsRequest<'a> {
    model: &'a str,
    input: TextOrArray<'a, InputItem<'a>>,
    questions: Vec<Question<'a>>,
}

#[derive(Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum Question<'a> {
    Predicate {
        name: String,
        instructions: TextOrArray<'a, InputPart<'a>>,
    },
    Choice {
        name: String,
        instructions: TextOrArray<'a, InputPart<'a>>,
        choices: Vec<Choice<'a>>,
    },
    Score {
        name: String,
        instructions: TextOrArray<'a, InputPart<'a>>,
        levels: Vec<Level<'a>>,
    },
}

#[derive(Serialize)]
struct Choice<'a> {
    value: ChoiceValue<'a>,
    #[serde(skip_serializing_if = "Option::is_none")]
    description: Option<Cow<'a, str>>,
}

#[derive(Serialize)]
#[serde(untagged)]
enum ChoiceValue<'a> {
    Label(&'a str),
    Boolean(bool),
}

#[derive(Serialize)]
struct Level<'a> {
    label: &'a str,
    #[serde(skip_serializing_if = "Option::is_none")]
    description: Option<Cow<'a, str>>,
}

pub(super) fn build_request<'a>(
    model: &'a str,
    request: &'a JudgmentRequest,
) -> Result<DecisionsRequest<'a>> {
    Ok(DecisionsRequest {
        model,
        input: input(&request.input)?,
        questions: named_questions(request)
            .map(|(name, _, question)| map_question(name, question))
            .collect::<Result<_>>()?,
    })
}

pub(super) fn named_questions(
    request: &JudgmentRequest,
) -> impl Iterator<Item = (String, &str, &JudgmentQuestion)> {
    request
        .questions
        .iter()
        .enumerate()
        .map(|(index, (id, question))| (format!("q{index}"), id.as_str(), question))
}

fn map_question(name: String, question: &JudgmentQuestion) -> Result<Question<'_>> {
    Ok(match question {
        JudgmentQuestion::Condition {
            instructions: content,
            criteria: None,
        } => Question::Predicate {
            name,
            instructions: instructions(content)?,
        },
        JudgmentQuestion::Condition {
            instructions: content,
            criteria: Some(criteria),
        } => Question::Choice {
            name,
            instructions: instructions(content)?,
            choices: vec![
                Choice {
                    value: ChoiceValue::Boolean(true),
                    description: description(criteria.yes.as_ref())?,
                },
                Choice {
                    value: ChoiceValue::Boolean(false),
                    description: description(criteria.no.as_ref())?,
                },
            ],
        },
        JudgmentQuestion::Choice {
            instructions: content,
            options,
        } => Question::Choice {
            name,
            instructions: instructions(content)?,
            choices: options
                .iter()
                .map(|option| {
                    Ok(Choice {
                        value: ChoiceValue::Label(&option.label),
                        description: description(option.description.as_ref())?,
                    })
                })
                .collect::<Result<_>>()?,
        },
        JudgmentQuestion::Score {
            instructions: content,
            levels,
        } => Question::Score {
            name,
            instructions: instructions(content)?,
            levels: levels
                .iter()
                .map(|level| {
                    Ok(Level {
                        label: &level.label,
                        description: description(level.description.as_ref())?,
                    })
                })
                .collect::<Result<_>>()?,
        },
    })
}

fn description(value: Option<&JudgmentDescription>) -> Result<Option<Cow<'_, str>>> {
    match value {
        Some(JudgmentDescription::Text(text)) => Ok(Some(Cow::Borrowed(text))),
        Some(JudgmentDescription::Json(json)) => Ok(Some(Cow::Owned(serde_json::to_string(json)?))),
        None => Ok(None),
    }
}
