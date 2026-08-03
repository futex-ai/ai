//! Reqwest request payload and redirect classification.

use reqwest::{Client, StatusCode};
use serde_json::Value;
use url::Url;

pub(super) enum RequestPayload {
    Get,
    Json(Value),
    Form(Vec<(String, String)>),
}

impl RequestPayload {
    pub(super) fn is_get(&self) -> bool {
        matches!(self, Self::Get)
    }
}

pub(super) fn request_builder(
    client: &Client,
    url: &Url,
    payload: &RequestPayload,
) -> reqwest::RequestBuilder {
    match payload {
        RequestPayload::Get => client.get(url.clone()),
        RequestPayload::Json(body) => client.post(url.clone()).json(body),
        RequestPayload::Form(fields) => client.post(url.clone()).form(fields),
    }
}

pub(super) fn follows_redirect(status: StatusCode) -> bool {
    matches!(
        status,
        StatusCode::MOVED_PERMANENTLY
            | StatusCode::FOUND
            | StatusCode::SEE_OTHER
            | StatusCode::TEMPORARY_REDIRECT
            | StatusCode::PERMANENT_REDIRECT
    )
}
