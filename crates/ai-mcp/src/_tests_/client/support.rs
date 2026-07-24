//! Deterministic transport support for client unit tests.

use std::{
    collections::{BTreeMap, VecDeque},
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, AtomicUsize, Ordering},
    },
    time::Duration,
};

use async_trait::async_trait;
use serde_json::Value;

use crate::{McpEventStream, McpHttpPayload, McpHttpResponse, McpHttpTransport, Result};

#[derive(Clone)]
pub(super) struct RecordedPost {
    pub(super) headers: BTreeMap<String, String>,
    pub(super) body: Value,
}

pub(super) struct ScriptedTransport {
    responses: Mutex<VecDeque<McpHttpResponse>>,
    posts: Mutex<Vec<RecordedPost>>,
    deletes: Mutex<Vec<BTreeMap<String, String>>>,
    side_reply_seen: Arc<AtomicBool>,
}

impl ScriptedTransport {
    pub(super) fn new(responses: Vec<McpHttpResponse>) -> Arc<Self> {
        Self::new_with_gate(responses, Arc::new(AtomicBool::new(false)))
    }

    pub(super) fn new_with_gate(
        responses: Vec<McpHttpResponse>,
        side_reply_seen: Arc<AtomicBool>,
    ) -> Arc<Self> {
        Arc::new(Self {
            responses: Mutex::new(responses.into()),
            posts: Mutex::new(Vec::new()),
            deletes: Mutex::new(Vec::new()),
            side_reply_seen,
        })
    }

    pub(super) fn posts(&self) -> Vec<RecordedPost> {
        self.posts.lock().unwrap().clone()
    }

    pub(super) fn delete_count(&self) -> usize {
        self.deletes.lock().unwrap().len()
    }

    fn next_response(&self) -> McpHttpResponse {
        self.responses
            .lock()
            .unwrap()
            .pop_front()
            .expect("scripted response")
    }
}

#[async_trait]
impl McpHttpTransport for ScriptedTransport {
    async fn post(
        &self,
        _url: &str,
        headers: &BTreeMap<String, String>,
        body: &Value,
        _max_response_bytes: usize,
        _timeout: Duration,
    ) -> Result<McpHttpResponse> {
        self.posts.lock().unwrap().push(RecordedPost {
            headers: headers.clone(),
            body: body.clone(),
        });
        if body.get("id").is_some()
            && body.get("method").is_none()
            && (body.get("result").is_some() || body.get("error").is_some())
        {
            self.side_reply_seen.store(true, Ordering::SeqCst);
        }
        Ok(self.next_response())
    }

    async fn delete(
        &self,
        _url: &str,
        headers: &BTreeMap<String, String>,
        _max_response_bytes: usize,
        _timeout: Duration,
    ) -> Result<McpHttpResponse> {
        self.deletes.lock().unwrap().push(headers.clone());
        Ok(self.next_response())
    }
}

pub(super) fn json_response(
    status: u16,
    body: Value,
    headers: BTreeMap<String, Vec<String>>,
) -> McpHttpResponse {
    McpHttpResponse {
        status,
        headers,
        payload: McpHttpPayload::Json(body),
    }
}

pub(super) fn empty_response(status: u16) -> McpHttpResponse {
    McpHttpResponse {
        status,
        headers: BTreeMap::new(),
        payload: McpHttpPayload::None,
    }
}

pub(super) fn event_response(events: Vec<Value>, gate: Arc<AtomicBool>) -> McpHttpResponse {
    McpHttpResponse {
        status: 200,
        headers: BTreeMap::new(),
        payload: McpHttpPayload::EventStream(Box::new(ScriptedEventStream {
            events: events.into(),
            gate,
            polls: AtomicUsize::new(0),
        })),
    }
}

struct ScriptedEventStream {
    events: VecDeque<Value>,
    gate: Arc<AtomicBool>,
    polls: AtomicUsize,
}

#[async_trait]
impl McpEventStream for ScriptedEventStream {
    async fn next_message(&mut self) -> Result<Option<Value>> {
        let poll = self.polls.fetch_add(1, Ordering::SeqCst);
        if poll > 0 {
            assert!(
                self.gate.load(Ordering::SeqCst),
                "side reply must finish before polling the original stream"
            );
        }
        Ok(self.events.pop_front())
    }
}
