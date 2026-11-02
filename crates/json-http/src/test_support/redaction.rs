//! Removal of credentials from requests, responses, and diagnostics.

use std::collections::BTreeMap;

use reqwest::header::HeaderValue;
use serde_json::Value;

use crate::{JsonHttpBody, JsonHttpHeaders, JsonHttpRequest};

use super::types::{MultipartSummary, RecordedBody, RecordedMethod, RecordedRequest};

const REDACTED: &str = "[redacted]";

/// Sanitizes known credentials and named credential fields before persistence.
#[derive(Clone, Default)]
pub struct Redactor {
    secrets: Vec<String>,
}

impl Redactor {
    /// Builds a sanitizer from additional credentials, such as the recorder key.
    pub fn new(secrets: impl IntoIterator<Item = String>) -> Self {
        let mut secrets: Vec<_> = secrets
            .into_iter()
            .filter(|secret| !secret.is_empty())
            .collect();
        secrets.sort_by_key(|secret| std::cmp::Reverse(secret.len()));
        secrets.dedup();
        Self { secrets }
    }

    pub(super) fn for_request(request: &JsonHttpRequest) -> Self {
        let mut secrets = Vec::new();
        for (name, value) in &request.headers {
            if sensitive(name) {
                secrets.push(value.clone());
                if let Some((_, token)) = value.split_once(' ') {
                    secrets.push(token.to_owned());
                }
            }
        }
        if let Ok(url) = reqwest::Url::parse(&request.url) {
            secrets.extend(
                url.query_pairs()
                    .filter(|(key, _)| sensitive(key))
                    .map(|(_, value)| value.into_owned()),
            );
            if let Some(password) = url.password() {
                secrets.push(password.to_owned());
            }
        }
        if let Some(bytes) = request.body.as_ref().and_then(JsonHttpBody::as_json_bytes)
            && let Ok(body) = serde_json::from_slice::<Value>(bytes)
        {
            collect_secrets(&body, &mut secrets);
        }
        Self::new(secrets)
    }

    /// Replaces known credentials in free text.
    pub fn text(&self, text: &str) -> String {
        self.secrets.iter().fold(text.to_owned(), |text, secret| {
            text.replace(secret, REDACTED)
        })
    }

    /// Recursively sanitizes strings and named credential fields.
    pub fn value(&self, value: &Value) -> Value {
        match value {
            Value::String(text) => Value::String(self.text(text)),
            Value::Array(values) => {
                Value::Array(values.iter().map(|value| self.value(value)).collect())
            }
            Value::Object(values) => Value::Object(
                values
                    .iter()
                    .map(|(key, value)| {
                        let value = if sensitive(key) {
                            Value::String(REDACTED.to_owned())
                        } else {
                            self.value(value)
                        };
                        (self.text(key), value)
                    })
                    .collect(),
            ),
            value => value.clone(),
        }
    }

    pub(super) fn event_data(&self, data: &str) -> String {
        match serde_json::from_str(data) {
            Ok(value) => self.value(&value).to_string(),
            Err(_) => self.text(data),
        }
    }

    pub(super) fn bytes(&self, bytes: &[u8]) -> Vec<u8> {
        let mut bytes = bytes.to_vec();
        for secret in &self.secrets {
            let mut start = 0;
            while let Some(offset) = bytes[start..]
                .windows(secret.len())
                .position(|part| part == secret.as_bytes())
            {
                let position = start + offset;
                bytes.splice(position..position + secret.len(), REDACTED.bytes());
                start = position + REDACTED.len();
            }
        }
        bytes
    }

    pub(super) fn headers(&self, headers: &JsonHttpHeaders) -> JsonHttpHeaders {
        let mut sanitized = JsonHttpHeaders::new();
        for (name, value) in headers {
            let bytes = if sensitive(name.as_str()) {
                REDACTED.as_bytes().to_vec()
            } else {
                self.bytes(value.as_bytes())
            };
            let value = HeaderValue::from_bytes(&bytes)
                .unwrap_or_else(|_| HeaderValue::from_static(REDACTED));
            sanitized.append(name.clone(), value);
        }
        sanitized
    }

    pub(super) fn request(&self, request: &JsonHttpRequest) -> RecordedRequest {
        let headers = request
            .headers
            .iter()
            .map(|(key, value)| {
                let value = if sensitive(key) {
                    REDACTED.to_owned()
                } else {
                    self.text(value)
                };
                (self.text(key), value)
            })
            .collect::<BTreeMap<_, _>>();
        let body = request.body.as_ref().map(|body| match body {
            JsonHttpBody::Json(bytes) => {
                let value = serde_json::from_slice(bytes)
                    .unwrap_or_else(|_| Value::String(String::from_utf8_lossy(bytes).into_owned()));
                RecordedBody::Json(self.value(&value))
            }
            JsonHttpBody::Multipart(form) => RecordedBody::Multipart(
                form.fields
                    .iter()
                    .map(|field| MultipartSummary {
                        name: self.text(&field.name),
                        filename: field.filename.as_ref().map(|value| self.text(value)),
                        content_type: field.content_type.as_ref().map(|value| self.text(value)),
                        size: field.bytes.len(),
                    })
                    .collect(),
            ),
        });
        RecordedRequest {
            method: RecordedMethod::from(request.method),
            url: self.url(&request.url),
            headers,
            body,
        }
    }

    fn url(&self, input: &str) -> String {
        let Ok(mut url) = reqwest::Url::parse(input) else {
            return self.text(input);
        };
        let pairs: Vec<_> = url
            .query_pairs()
            .map(|(key, value)| {
                let value = if sensitive(&key) {
                    REDACTED.to_owned()
                } else {
                    self.text(&value)
                };
                (key.into_owned(), value)
            })
            .collect();
        if !url.username().is_empty() {
            let _ = url.set_username(REDACTED);
        }
        if url.password().is_some() {
            let _ = url.set_password(Some(REDACTED));
        }
        if url.query().is_some() {
            url.query_pairs_mut().clear().extend_pairs(pairs);
        }
        self.text(url.as_str())
    }
}

fn sensitive(name: &str) -> bool {
    matches!(
        name.to_ascii_lowercase().replace('_', "-").as_str(),
        "authorization"
            | "proxy-authorization"
            | "x-api-key"
            | "x-goog-api-key"
            | "api-key"
            | "key"
            | "token"
            | "access-token"
            | "refresh-token"
            | "password"
            | "secret"
            | "client-secret"
            | "cookie"
            | "set-cookie"
    )
}

fn collect_secrets(value: &Value, secrets: &mut Vec<String>) {
    match value {
        Value::Object(values) => {
            for (name, value) in values {
                if sensitive(name)
                    && let Some(secret) = value.as_str()
                {
                    secrets.push(secret.to_owned());
                }
                collect_secrets(value, secrets);
            }
        }
        Value::Array(values) => {
            for value in values {
                collect_secrets(value, secrets);
            }
        }
        _ => {}
    }
}
