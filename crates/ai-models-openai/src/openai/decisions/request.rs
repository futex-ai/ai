//! OpenAI Decisions request mapping.

use ai_interface::{
    JudgmentConditionCriteria, JudgmentContent, JudgmentError, JudgmentQuestion, JudgmentRequest,
    JudgmentResult,
};
use serde::Serialize;

const TRUE_WHEN: &str = "True when: ";
const FALSE_WHEN: &str = "False when: ";

/// Serialized Decisions request body.
#[derive(Debug, Serialize)]
pub(super) struct DecisionsRequest<'a> {
    model: &'a str,
    input: String,
    questions: Vec<DecisionsQuestion<'a>>,
}

#[derive(Debug, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum DecisionsQuestion<'a> {
    Predicate {
        name: &'a str,
        instructions: String,
    },
    Choice {
        name: &'a str,
        instructions: String,
        choices: Vec<DecisionsChoice>,
    },
    Score {
        name: &'a str,
        instructions: String,
        levels: Vec<DecisionsLevel>,
    },
}

#[derive(Debug, Serialize)]
struct DecisionsChoice {
    value: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    description: Option<String>,
}

#[derive(Debug, Serialize)]
struct DecisionsLevel {
    label: String,
}

/// Maps a shared judgment request to the Decisions wire contract.
///
/// Questions are emitted in ascending id order and carry the caller id as
/// the OpenAI `name` so answers can be matched by name.
pub(super) fn build_request<'a>(
    model_id: &'a str,
    request: &'a JudgmentRequest,
) -> JudgmentResult<DecisionsRequest<'a>> {
    let mut questions = Vec::with_capacity(request.questions.len());
    for (id, question) in &request.questions {
        questions.push(map_question(id, question)?);
    }
    Ok(DecisionsRequest {
        model: model_id,
        input: render_content(&request.state)?,
        questions,
    })
}

/// Renders shared content as the text the Decisions API accepts.
///
/// Text is verbatim; objects and arrays become compact JSON.
fn render_content(content: &JudgmentContent) -> JudgmentResult<String> {
    match content {
        JudgmentContent::Text(text) => Ok(text.clone()),
        JudgmentContent::Object(_) | JudgmentContent::Array(_) => {
            match serde_json::to_string(content) {
                Ok(text) => Ok(text),
                Err(source) => Err(JudgmentError::internal(source)),
            }
        }
    }
}

fn render_optional(content: Option<&JudgmentContent>) -> JudgmentResult<Option<String>> {
    match content {
        Some(content) => Ok(Some(render_content(content)?)),
        None => Ok(None),
    }
}

fn map_question<'a>(
    id: &'a str,
    question: &JudgmentQuestion,
) -> JudgmentResult<DecisionsQuestion<'a>> {
    match question {
        JudgmentQuestion::Condition {
            instructions,
            criteria,
        } => Ok(DecisionsQuestion::Predicate {
            name: id,
            instructions: predicate_instructions(instructions, criteria.as_ref())?,
        }),
        JudgmentQuestion::Choice {
            instructions,
            options,
        } => {
            let mut choices = Vec::with_capacity(options.len());
            for (label, description) in options {
                choices.push(DecisionsChoice {
                    value: label.clone(),
                    description: render_optional(description.as_ref())?,
                });
            }
            Ok(DecisionsQuestion::Choice {
                name: id,
                instructions: render_content(instructions)?,
                choices,
            })
        }
        JudgmentQuestion::Score {
            instructions,
            levels,
        } => {
            let mut wire_levels = Vec::with_capacity(levels.len());
            for level in levels {
                wire_levels.push(DecisionsLevel {
                    label: render_content(level)?,
                });
            }
            Ok(DecisionsQuestion::Score {
                name: id,
                instructions: render_content(instructions)?,
                levels: wire_levels,
            })
        }
    }
}

/// Folds optional condition criteria into the predicate instructions.
///
/// The Decisions predicate has no criteria field, so present descriptions
/// are appended after a blank line as `True when:` and `False when:` lines.
fn predicate_instructions(
    instructions: &JudgmentContent,
    criteria: Option<&JudgmentConditionCriteria>,
) -> JudgmentResult<String> {
    let mut text = render_content(instructions)?;
    let Some(criteria) = criteria else {
        return Ok(text);
    };
    let mut lines = Vec::new();
    if let Some(yes) = &criteria.yes {
        lines.push([TRUE_WHEN, &render_content(yes)?].concat());
    }
    if let Some(no) = &criteria.no {
        lines.push([FALSE_WHEN, &render_content(no)?].concat());
    }
    if lines.is_empty() {
        return Ok(text);
    }
    text.push_str("\n\n");
    text.push_str(&lines.join("\n"));
    Ok(text)
}

#[cfg(test)]
#[path = "_tests_/request_tests.rs"]
mod request_tests;
