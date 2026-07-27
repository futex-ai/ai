//! System-proxy isolation coverage for the production OAuth transport.

use std::{
    io::{Read, Write},
    net::{IpAddr, Ipv4Addr, TcpListener},
    process::Command,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    thread,
    time::{Duration, Instant},
};

use ai_mcp_oauth::{
    OAuthDnsResolver, OAuthEndpointKind, OAuthHttpLimits, OAuthHttpTransport, OAuthUrlPolicy,
    ReqwestOAuthHttpTransport,
};
use async_trait::async_trait;

const CHILD_MARKER: &str = "AI_MCP_OAUTH_PROXY_TEST_CHILD";
const ORIGIN_PORT: &str = "AI_MCP_OAUTH_PROXY_TEST_ORIGIN_PORT";

#[test]
fn ignores_system_proxy_environment() {
    let origin = TcpListener::bind((Ipv4Addr::LOCALHOST, 0)).unwrap();
    let origin_port = origin.local_addr().unwrap().port();
    let proxy = TcpListener::bind((Ipv4Addr::LOCALHOST, 0)).unwrap();
    let proxy_url = format!("http://{}", proxy.local_addr().unwrap());
    let finished = Arc::new(AtomicBool::new(false));
    let origin_thread = serve_once(origin, "200 OK", r#"{"ok":true}"#, Arc::clone(&finished));
    let proxy_thread = serve_once(
        proxy,
        "502 Bad Gateway",
        r#"{"proxy":true}"#,
        Arc::clone(&finished),
    );

    let output = Command::new(std::env::current_exe().unwrap())
        .args(["--exact", "proxy_isolation_child", "--nocapture"])
        .env(CHILD_MARKER, "1")
        .env(ORIGIN_PORT, origin_port.to_string())
        .env("HTTP_PROXY", &proxy_url)
        .env("http_proxy", &proxy_url)
        .env("HTTPS_PROXY", &proxy_url)
        .env("https_proxy", &proxy_url)
        .env("ALL_PROXY", &proxy_url)
        .env("all_proxy", &proxy_url)
        .env_remove("NO_PROXY")
        .env_remove("no_proxy")
        .output()
        .unwrap();
    finished.store(true, Ordering::SeqCst);
    let origin_connected = origin_thread.join().unwrap();
    let proxy_connected = proxy_thread.join().unwrap();

    assert!(
        output.status.success(),
        "child failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(origin_connected, "the pinned origin was not contacted");
    assert!(!proxy_connected, "the system proxy was contacted");
}

#[tokio::test]
async fn proxy_isolation_child() {
    if std::env::var_os(CHILD_MARKER).is_none() {
        return;
    }
    let port = std::env::var(ORIGIN_PORT).unwrap();
    let transport = ReqwestOAuthHttpTransport::with_resolver(Arc::new(LoopbackResolver));
    let response = transport
        .post_form(
            &format!("http://proxy-test.localhost:{port}/token"),
            OAuthEndpointKind::Token,
            &OAuthUrlPolicy::loopback_development(),
            OAuthHttpLimits {
                timeout: Duration::from_secs(2),
                max_response_bytes: 1024,
                max_redirects: 1,
            },
            &[("refresh_token".to_owned(), "test-secret".to_owned())],
        )
        .await
        .unwrap();

    assert_eq!(response.status, 200);
}

struct LoopbackResolver;

#[async_trait]
impl OAuthDnsResolver for LoopbackResolver {
    async fn resolve(&self, _host: &str, _port: u16) -> ai_mcp_oauth::Result<Vec<IpAddr>> {
        Ok(vec![IpAddr::V4(Ipv4Addr::LOCALHOST)])
    }
}

fn serve_once(
    listener: TcpListener,
    status: &'static str,
    body: &'static str,
    finished: Arc<AtomicBool>,
) -> thread::JoinHandle<bool> {
    listener.set_nonblocking(true).unwrap();
    thread::spawn(move || {
        let deadline = Instant::now() + Duration::from_secs(5);
        while !finished.load(Ordering::SeqCst) && Instant::now() < deadline {
            match listener.accept() {
                Ok((mut stream, _)) => {
                    stream
                        .set_read_timeout(Some(Duration::from_secs(1)))
                        .unwrap();
                    let mut request = [0_u8; 4096];
                    let _ = stream.read(&mut request);
                    let response = format!(
                        "HTTP/1.1 {status}\r\nContent-Type: application/json\r\n\
                         Content-Length: {}\r\nConnection: close\r\n\r\n{body}",
                        body.len()
                    );
                    stream.write_all(response.as_bytes()).unwrap();
                    return true;
                }
                Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                    thread::sleep(Duration::from_millis(10));
                }
                Err(error) => panic!("listener failed: {error}"),
            }
        }
        false
    })
}
