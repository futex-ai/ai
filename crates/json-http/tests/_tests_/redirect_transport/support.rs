//! Local HTTP servers for credential-bearing redirect chains.

use std::{
    collections::BTreeMap,
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    },
};

use async_trait::async_trait;
use json_http::{JsonHttpAuth, Result};

use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{TcpListener, TcpStream},
    task::JoinHandle,
};

pub(super) struct CountingAuth {
    pub(super) calls: Arc<AtomicUsize>,
}

#[async_trait]
impl JsonHttpAuth for CountingAuth {
    async fn apply_headers(&self, headers: &mut BTreeMap<String, String>) -> Result<()> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        headers.insert("X-Session-Key".to_owned(), "hook-secret".to_owned());
        headers.insert("X-Changed".to_owned(), "changed-secret".to_owned());
        Ok(())
    }
}

pub(super) struct WireRequest {
    pub(super) method: String,
    pub(super) headers: BTreeMap<String, String>,
    pub(super) body: Vec<u8>,
}

pub(super) struct Reply {
    pub(super) status: &'static str,
    pub(super) location: Option<String>,
    pub(super) content_type: &'static str,
    pub(super) body: &'static str,
}

impl Reply {
    pub(super) fn redirect(location: String, status: &'static str) -> Self {
        Self {
            status,
            location: Some(location),
            content_type: "text/plain",
            body: "",
        }
    }

    pub(super) fn success(content_type: &'static str, body: &'static str) -> Self {
        Self {
            status: "200 OK",
            location: None,
            content_type,
            body,
        }
    }
}

pub(super) async fn listener() -> (TcpListener, String) {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let url = format!("http://{}", listener.local_addr().unwrap());
    (listener, url)
}

pub(super) fn serve(listener: TcpListener, replies: Vec<Reply>) -> JoinHandle<Vec<WireRequest>> {
    tokio::spawn(async move {
        let mut requests = Vec::new();
        for reply in replies {
            let (mut socket, _) = listener.accept().await.unwrap();
            requests.push(read_request(&mut socket).await);
            let location = reply
                .location
                .map(|url| format!("Location: {url}\r\n"))
                .unwrap_or_default();
            let response = format!(
                "HTTP/1.1 {}\r\nConnection: close\r\n{location}Content-Type: {}\r\nContent-Length: {}\r\n\r\n{}",
                reply.status,
                reply.content_type,
                reply.body.len(),
                reply.body,
            );
            socket.write_all(response.as_bytes()).await.unwrap();
            socket.shutdown().await.unwrap();
        }
        requests
    })
}

async fn read_request(socket: &mut TcpStream) -> WireRequest {
    let mut bytes = Vec::new();
    let (head_end, length) = loop {
        let mut buffer = [0; 4096];
        let count = socket.read(&mut buffer).await.unwrap();
        assert_ne!(count, 0, "request ended before its headers");
        bytes.extend_from_slice(&buffer[..count]);
        if let Some(end) = bytes.windows(4).position(|part| part == b"\r\n\r\n") {
            let head = String::from_utf8_lossy(&bytes[..end]);
            let length = head
                .lines()
                .filter_map(|line| line.split_once(':'))
                .find(|(name, _)| name.eq_ignore_ascii_case("content-length"))
                .map(|(_, length)| length.trim().parse::<usize>().unwrap())
                .unwrap_or(0);
            break (end + 4, length);
        }
    };
    while bytes.len() < head_end + length {
        let mut buffer = [0; 4096];
        let count = socket.read(&mut buffer).await.unwrap();
        assert_ne!(count, 0, "request body ended early");
        bytes.extend_from_slice(&buffer[..count]);
    }
    let head = String::from_utf8_lossy(&bytes[..head_end]);
    let method = head.split_whitespace().next().unwrap().to_owned();
    let headers = head
        .lines()
        .skip(1)
        .filter_map(|line| line.split_once(':'))
        .map(|(name, value)| (name.to_ascii_lowercase(), value.trim().to_owned()))
        .collect();
    WireRequest {
        method,
        headers,
        body: bytes[head_end..head_end + length].to_vec(),
    }
}
