//! Decisions response envelope, answer names, and token usage.

use std::collections::BTreeMap;

use ai_interface::{JudgmentRequest, JudgmentResponse, JudgmentResult, ModelUsage};
use ai_models_core::ErrorContext;
use serde::Deserialize;
use serde_json::value::RawValue;

use crate::PROVIDER_ID;

use super::{
    answer::normalize_answer,
    error::{decode, invalid_output},
    request::named_questions,
};

#[derive(Deserialize)]
struct DecisionsResponse<'a> {
    model: String,
    #[serde(borrow)]
    answers: Vec<&'a RawValue>,
    usage: Option<Usage>,
}

#[derive(Deserialize)]
struct AnswerName {
    name: Option<String>,
}

pub(super) fn parse_response(
    context: &ErrorContext,
    model_id: &str,
    request: &JudgmentRequest,
    body: &[u8],
) -> JudgmentResult<JudgmentResponse> {
    let decoded: DecisionsResponse<'_> = decode(context, body)?;
    let mut named = index_answers(context, decoded.answers)?;
    let mut answers = BTreeMap::new();
    for (name, id, question) in named_questions(request) {
        if let Some(body) = named.remove(&name) {
            let answer = normalize_answer(context, model_id, id, question, body)?;
            answers.insert(id.to_owned(), answer);
        }
    }
    let response = JudgmentResponse {
        provider: PROVIDER_ID.to_string(),
        model_id: model_id.to_owned(),
        resolved_model_id: decoded.model,
        answers,
        usage: decoded.usage.unwrap_or_default().normalize(),
    };
    response.validate_against(request)?;
    Ok(response)
}

fn index_answers<'a>(
    context: &ErrorContext,
    answers: Vec<&'a RawValue>,
) -> JudgmentResult<BTreeMap<String, &'a RawValue>> {
    let mut by_name = BTreeMap::new();
    for answer in answers {
        let json = answer.get();
        if !json.starts_with('{') {
            return Err(invalid_output(context, "answer must be a JSON object"));
        }
        let AnswerName { name } = decode(context, json.as_bytes())?;
        if let Some(name) = name
            && by_name.insert(name, answer).is_some()
        {
            return Err(invalid_output(context, "duplicate answer name"));
        }
    }
    Ok(by_name)
}

#[derive(Default, Deserialize)]
struct Usage {
    input_tokens: Option<u64>,
    input_tokens_details: Option<InputTokenDetails>,
    output_tokens: Option<u64>,
    output_tokens_details: Option<OutputTokenDetails>,
    total_tokens: Option<u64>,
}

#[derive(Deserialize)]
struct InputTokenDetails {
    cached_tokens: Option<u64>,
}

#[derive(Deserialize)]
struct OutputTokenDetails {
    reasoning_tokens: Option<u64>,
}

impl Usage {
    fn normalize(self) -> ModelUsage {
        let cached_input_tokens = self
            .input_tokens_details
            .and_then(|details| details.cached_tokens)
            .unwrap_or_default();
        let reasoning_tokens = self
            .output_tokens_details
            .and_then(|details| details.reasoning_tokens)
            .unwrap_or_default();
        let input_tokens = self
            .input_tokens
            .unwrap_or_default()
            .saturating_sub(cached_input_tokens);
        let output_tokens = self
            .output_tokens
            .unwrap_or_default()
            .saturating_sub(reasoning_tokens);
        let total_tokens = self.total_tokens.unwrap_or_else(|| {
            input_tokens
                .saturating_add(cached_input_tokens)
                .saturating_add(output_tokens)
                .saturating_add(reasoning_tokens)
        });
        ModelUsage {
            input_tokens,
            output_tokens,
            cached_input_tokens,
            reasoning_tokens,
            total_tokens,
            ..ModelUsage::default()
        }
    }
}
