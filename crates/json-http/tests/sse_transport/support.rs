use std::time::Duration;

use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpListener,
    task::JoinHandle,
    time::sleep,
};

pub(super) enum BodyFraming {
    Chunked { graceful: bool },
    Fixed { declared_length: usize },
}

pub(super) enum ResponseStep {
    Bytes(Vec<u8>),
    Delay(Duration),
}

pub(super) struct ResponseSpec {
    pub(super) status: &'static str,
    pub(super) content_type: Option<&'static str>,
    pub(super) header_delay: Duration,
    pub(super) framing: BodyFraming,
    pub(super) steps: Vec<ResponseStep>,
}

impl ResponseSpec {
    pub(super) fn sse(steps: Vec<ResponseStep>) -> Self {
        Self {
            status: "200 OK",
            content_type: Some("text/event-stream; charset=utf-8"),
            header_delay: Duration::ZERO,
            framing: BodyFraming::Chunked { graceful: true },
            steps,
        }
    }
}

pub(super) struct TestServer {
    pub(super) url: String,
    handle: JoinHandle<String>,
}

impl TestServer {
    pub(super) async fn request(self) -> String {
        self.handle.await.unwrap()
    }
}

pub(super) async fn spawn_server(spec: ResponseSpec) -> TestServer {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let handle = tokio::spawn(async move {
        let (mut socket, _) = listener.accept().await.unwrap();
        let request = read_request(&mut socket).await;
        sleep(spec.header_delay).await;
        let header = response_header(&spec);
        if socket.write_all(header.as_bytes()).await.is_err() {
            return request;
        }

        for step in spec.steps {
            match step {
                ResponseStep::Bytes(bytes) => {
                    if write_body_bytes(&mut socket, &spec.framing, &bytes)
                        .await
                        .is_err()
                    {
                        return request;
                    }
                }
                ResponseStep::Delay(duration) => sleep(duration).await,
            }
        }

        if matches!(spec.framing, BodyFraming::Chunked { graceful: true }) {
            let _ = socket.write_all(b"0\r\n\r\n").await;
        }
        let _ = socket.shutdown().await;
        request
    });

    TestServer {
        url: format!("http://{address}/stream"),
        handle,
    }
}

async fn read_request(socket: &mut tokio::net::TcpStream) -> String {
    let mut request = Vec::new();
    let mut buffer = [0_u8; 1024];
    loop {
        let count = socket.read(&mut buffer).await.unwrap();
        if count == 0 {
            break;
        }
        request.extend_from_slice(&buffer[..count]);
        if request.windows(4).any(|window| window == b"\r\n\r\n") {
            break;
        }
    }
    String::from_utf8_lossy(&request).into_owned()
}

fn response_header(spec: &ResponseSpec) -> String {
    let mut header = format!("HTTP/1.1 {}\r\nConnection: close\r\n", spec.status);
    if let Some(content_type) = spec.content_type {
        header.push_str(&format!("Content-Type: {content_type}\r\n"));
    }
    match spec.framing {
        BodyFraming::Chunked { .. } => header.push_str("Transfer-Encoding: chunked\r\n\r\n"),
        BodyFraming::Fixed { declared_length } => {
            header.push_str(&format!("Content-Length: {declared_length}\r\n\r\n"));
        }
    }
    header
}

async fn write_body_bytes(
    socket: &mut tokio::net::TcpStream,
    framing: &BodyFraming,
    bytes: &[u8],
) -> std::io::Result<()> {
    match framing {
        BodyFraming::Chunked { .. } => {
            socket
                .write_all(format!("{:x}\r\n", bytes.len()).as_bytes())
                .await?;
            socket.write_all(bytes).await?;
            socket.write_all(b"\r\n").await
        }
        BodyFraming::Fixed { .. } => socket.write_all(bytes).await,
    }
}
