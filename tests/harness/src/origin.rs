use std::net::SocketAddr;
use std::time::{Duration, Instant};

use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio_stream::StreamExt;

use crate::http::{client_via_proxy, http_send_get};

const BUDGET: Duration = Duration::from_secs(30);

pub async fn start_origin() -> Result<SocketAddr, String> {
    start_origin_on(SocketAddr::from((std::net::Ipv4Addr::LOCALHOST, 0))).await
}

pub async fn start_origin_v6() -> Result<SocketAddr, String> {
    start_origin_on(SocketAddr::from((std::net::Ipv6Addr::LOCALHOST, 0))).await
}

async fn start_origin_on(bind: SocketAddr) -> Result<SocketAddr, String> {
    let listener = TcpListener::bind(bind)
        .await
        .map_err(|e| format!("origin bind failed: {e}"))?;
    let addr = listener
        .local_addr()
        .map_err(|e| format!("origin local addr failed: {e}"))?;
    tokio::spawn(async move {
        loop {
            let Ok((stream, peer)) = listener.accept().await else {
                return;
            };
            tokio::spawn(serve_origin(stream, peer));
        }
    });
    Ok(addr)
}

#[must_use]
pub fn origin_url(origin: SocketAddr, route: &str) -> String {
    format!("http://{origin}{route}")
}

async fn read_head(stream: &mut TcpStream) -> Result<String, String> {
    let mut head = Vec::new();
    let mut byte = [0u8; 1];
    loop {
        let read = stream.read(&mut byte).await;
        let Ok(1) = read else {
            return Err("origin head read failed".to_owned());
        };
        head.push(byte[0]);
        if head.ends_with(b"\r\n\r\n") {
            let Ok(text) = String::from_utf8(head) else {
                return Err("origin head not utf8".to_owned());
            };
            return Ok(text);
        }
        if head.len() > 16 * 1024 {
            return Err("origin head too large".to_owned());
        }
    }
}

fn route_of(head: &str) -> &'static str {
    for route in ["peer", "hello", "beat", "gap", "stall", "quiet"] {
        if head.contains(format!("/{route}").as_str()) {
            return route;
        }
    }
    "unknown"
}

async fn serve_origin(stream: TcpStream, peer: SocketAddr) {
    let mut buffered = stream;
    let Ok(head) = read_head(&mut buffered).await else {
        return;
    };
    match route_of(&head) {
        "peer" => simple(&mut buffered, &peer.ip().to_string()).await,
        "hello" => simple(&mut buffered, "hello-origin").await,
        "beat" => beat(&mut buffered).await,
        "gap" => gap(&mut buffered).await,
        "stall" => stall(&mut buffered).await,
        "quiet" => quiet(&mut buffered).await,
        _ => simple(&mut buffered, "unknown-route").await,
    }
}

async fn simple(stream: &mut TcpStream, body: &str) {
    write_head(stream, body.len()).await;
    let _written = stream.write_all(body.as_bytes()).await.is_ok();
    let _shutdown = stream.shutdown().await.is_ok();
}

async fn write_head(stream: &mut TcpStream, len: usize) {
    let head = format!("HTTP/1.1 200 OK\r\nContent-Length: {len}\r\n\r\n");
    let _written = stream.write_all(head.as_bytes()).await.is_ok();
    let _flushed = stream.flush().await.is_ok();
}

async fn beat(stream: &mut TcpStream) {
    write_head(stream, 135).await;
    for n in 0..15 {
        let line = format!("chunk-{n:02}\n");
        let _written = stream.write_all(line.as_bytes()).await.is_ok();
        let _flushed = stream.flush().await.is_ok();
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    let _shutdown = stream.shutdown().await.is_ok();
}

async fn gap(stream: &mut TcpStream) {
    write_head(stream, 60).await;
    for _n in 0..60 {
        let _written = stream.write_all(b"x").await.is_ok();
        let _flushed = stream.flush().await.is_ok();
        tokio::time::sleep(Duration::from_millis(2)).await;
    }
    let _shutdown = stream.shutdown().await.is_ok();
}

async fn stall(stream: &mut TcpStream) {
    write_head(stream, 35).await;
    let _written = stream.write_all(b"first-half-line!\n").await.is_ok();
    let _flushed = stream.flush().await.is_ok();
    tokio::time::sleep(Duration::from_millis(500)).await;
    let _written = stream.write_all(b"second-half-line!\n").await.is_ok();
    let _shutdown = stream.shutdown().await.is_ok();
}

async fn quiet(stream: &mut TcpStream) {
    write_head(stream, 10).await;
    let _written = stream.write_all(b"tick\n").await.is_ok();
    let _flushed = stream.flush().await.is_ok();
    tokio::time::sleep(Duration::from_secs(2)).await;
    let _written = stream.write_all(b"late\n").await.is_ok();
    let _shutdown = stream.shutdown().await.is_ok();
}

pub async fn get_via_proxy(
    proxy_port: u16,
    origin: SocketAddr,
    route: &str,
) -> Result<(u16, String), String> {
    let client = client_via_proxy(&format!("http://127.0.0.1:{proxy_port}"))?;
    crate::realtime::timeout_real(http_send_get(&client, &origin_url(origin, route)), BUDGET)
        .await?
}

pub async fn first_data_delay(
    proxy_port: u16,
    origin: SocketAddr,
    route: &str,
) -> Result<(Duration, String), String> {
    let client = client_via_proxy(&format!("http://127.0.0.1:{proxy_port}"))?;
    crate::realtime::timeout_real(collect_delay(&client, &origin_url(origin, route)), BUDGET)
        .await?
}

pub async fn collect_delay(
    client: &reqwest::Client,
    url: &str,
) -> Result<(Duration, String), String> {
    let start = Instant::now();
    let response = client
        .get(url)
        .send()
        .await
        .map_err(|e| format!("GET {url} via proxy failed: {e}"))?;
    let mut streamed = response.bytes_stream();
    let mut body = String::new();
    let mut first = Option::<Duration>::None;
    while let Some(Ok(chunk)) = streamed.next().await {
        if first.is_none() {
            first = Some(start.elapsed());
        }
        body.push_str(&String::from_utf8_lossy(&chunk));
    }
    Ok((first.unwrap_or_else(|| start.elapsed()), body))
}

pub async fn collect_stream(response: reqwest::Response) -> String {
    let mut body = String::new();
    let mut streamed = response.bytes_stream();
    loop {
        let next = tokio::time::timeout(Duration::from_secs(5), streamed.next()).await;
        let Ok(Some(Ok(chunk))) = next else {
            break;
        };
        body.push_str(&String::from_utf8_lossy(&chunk));
    }
    body
}
