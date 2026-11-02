//! Redirect execution with the original credential origin and one timeout budget.

use std::{cell::RefCell, collections::BTreeSet};

use reqwest::{
    Client, Method, Request, RequestBuilder, Response, Url,
    header::{HeaderMap, REFERER},
};
use tokio::time::Instant;

use crate::request_headers::credential_header;

const PAYLOAD_HEADERS: &[&str] = &[
    "content-type",
    "content-length",
    "content-encoding",
    "transfer-encoding",
];

tokio::task_local! {
    static REDIRECTS: RefCell<RedirectState>;
}

#[derive(Default)]
struct RedirectState {
    count: usize,
    target: Option<Url>,
}

pub(crate) fn policy() -> reqwest::redirect::Policy {
    reqwest::redirect::Policy::custom(|attempt| {
        let limited = REDIRECTS
            .try_with(|state| {
                let mut state = state.borrow_mut();
                if state.count >= 10 {
                    true
                } else {
                    state.count += 1;
                    state.target = Some(attempt.url().clone());
                    false
                }
            })
            .unwrap_or(true);
        if limited {
            attempt.error("too many redirects")
        } else {
            attempt.stop()
        }
    })
}

pub(crate) async fn send(
    builder: RequestBuilder,
    auth_names: &BTreeSet<String>,
) -> reqwest::Result<Response> {
    let (client, request) = builder.build_split();
    let request = request?;
    REDIRECTS
        .scope(RefCell::default(), send_hops(client, request, auth_names))
        .await
}

async fn send_hops(
    client: Client,
    mut request: Request,
    auth_names: &BTreeSet<String>,
) -> reqwest::Result<Response> {
    let original = request.url().clone();
    let mut auth = request
        .headers()
        .iter()
        .filter(|(name, _)| {
            auth_names
                .iter()
                .any(|key| key.eq_ignore_ascii_case(name.as_str()))
                || credential_header(name.as_str())
        })
        .map(|(name, value)| (name.clone(), value.clone()))
        .collect::<HeaderMap>();
    let budget = request.timeout().copied();
    let started = Instant::now();
    loop {
        let previous = request.url().clone();
        let mut empty = Request::new(request.method().clone(), previous.clone());
        *empty.headers_mut() = request.headers().clone();
        *empty.version_mut() = request.version();
        let replay = request.try_clone();
        if let Some(budget) = budget {
            *request.timeout_mut() = Some(budget.saturating_sub(started.elapsed()));
        }
        let response = client.execute(request).await?;
        let target = REDIRECTS.with(|state| state.borrow_mut().target.take());
        let Some(target) = target else {
            return Ok(response);
        };
        let drop_body = response.status().as_u16() == 303
            || matches!(response.status().as_u16(), 301 | 302) && empty.method() == Method::POST;
        request = if drop_body {
            *empty.method_mut() = Method::GET;
            for name in PAYLOAD_HEADERS {
                empty.headers_mut().remove(*name);
                auth.remove(*name);
            }
            empty
        } else if let Some(replay) = replay {
            replay
        } else {
            return Ok(response);
        };
        *request.url_mut() = target.clone();
        for name in auth.keys() {
            request.headers_mut().remove(name);
        }
        if target.origin() == original.origin() {
            request.headers_mut().extend(auth.clone());
        }
        set_referer(&mut request, &previous);
    }
}

fn set_referer(request: &mut Request, previous: &Url) {
    if request.url().scheme() == "http" && previous.scheme() == "https" {
        return;
    }
    let mut referer = previous.clone();
    let _ = referer.set_username("");
    let _ = referer.set_password(None);
    referer.set_fragment(None);
    if let Ok(value) = referer.as_str().parse() {
        request.headers_mut().insert(REFERER, value);
    }
}
