use anybot_blob as _;
use anybot_bus as _;
use anybot_channels as _;
use anybot_domain as _;
use anybot_foundation as _;
use anybot_messages as _;
use anybot_protocol as _;
use anybot_store_server as _;
use argon2 as _;
use futures_util as _;
use getrandom as _;
use http_body_util as _;
use serde as _;
use serde_json as _;
use sha2 as _;
use socket2 as _;
use sqlx as _;
use time as _;
use tokio_stream as _;
use tracing as _;
use url as _;
use utoipa as _;
use uuid as _;

use std::net::{IpAddr, SocketAddr};
use std::time::Duration;

use axum::serve::Listener;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};

use anybot_server::conn_limit::PUBLIC_PEER_CONNECTIONS;
use anybot_server::serve_all::serve_listeners;
use anybot_server::{AppState, build_router};
use test_harness::timeout_real;

const WAIT: Duration = Duration::from_secs(5);
const HEALTH: &[u8] = b"GET /v1/health HTTP/1.1\r\nHost: localhost\r\n\r\n";

struct SeenFrom {
    inner: TcpListener,
    peer: Option<IpAddr>,
}

impl Listener for SeenFrom {
    type Io = TcpStream;
    type Addr = SocketAddr;

    async fn accept(&mut self) -> (TcpStream, SocketAddr) {
        let (io, addr) = Listener::accept(&mut self.inner).await;
        let shown = self
            .peer
            .map_or(addr, |ip| SocketAddr::new(ip, addr.port()));
        (io, shown)
    }

    fn local_addr(&self) -> std::io::Result<SocketAddr> {
        Listener::local_addr(&self.inner)
    }
}

async fn start(peers: [Option<IpAddr>; 2]) -> Result<[SocketAddr; 2], String> {
    let mut listeners = Vec::new();
    let mut addrs = Vec::new();
    for peer in peers {
        let inner = TcpListener::bind("127.0.0.1:0")
            .await
            .map_err(|e| e.to_string())?;
        addrs.push(inner.local_addr().map_err(|e| e.to_string())?);
        listeners.push(SeenFrom { inner, peer });
    }
    let app = build_router(AppState::new("test-9.9.9"));
    tokio::spawn(async move { serve_listeners(listeners, app, None).await });
    addrs
        .try_into()
        .map_err(|_e| "two addresses expected".to_owned())
}

fn header_end(buf: &[u8]) -> Option<usize> {
    buf.windows(4)
        .position(|w| w == b"\r\n\r\n")
        .map(|p| p.saturating_add(4))
}

fn content_length(head: &str) -> usize {
    head.lines()
        .filter_map(|line| line.split_once(':'))
        .find(|(name, _)| name.eq_ignore_ascii_case("content-length"))
        .and_then(|(_, value)| value.trim().parse().ok())
        .unwrap_or(0)
}

async fn read_response(stream: &mut TcpStream) -> Result<String, String> {
    let mut buf = Vec::new();
    let mut chunk = [0_u8; 4096];
    loop {
        if let Some(end) = header_end(&buf) {
            let head = String::from_utf8_lossy(buf.get(..end).unwrap_or_default()).into_owned();
            if buf.len() >= end.saturating_add(content_length(&head)) {
                return Ok(head);
            }
        }
        let read = stream.read(&mut chunk).await.map_err(|e| e.to_string())?;
        if read == 0 {
            return Err("connection closed before a full response".to_owned());
        }
        buf.extend_from_slice(chunk.get(..read).unwrap_or_default());
    }
}

async fn open_and_ask(addr: SocketAddr) -> Result<TcpStream, String> {
    let mut stream = TcpStream::connect(addr).await.map_err(|e| e.to_string())?;
    stream.write_all(HEALTH).await.map_err(|e| e.to_string())?;
    let head = timeout_real(read_response(&mut stream), WAIT).await??;
    assert!(head.starts_with("HTTP/1.1 200"), "{head}");
    Ok(stream)
}

async fn open_many(addr: SocketAddr, count: usize) -> Result<Vec<TcpStream>, String> {
    let mut held = Vec::new();
    for _ in 0..count {
        held.push(open_and_ask(addr).await?);
    }
    Ok(held)
}

async fn refused(addr: SocketAddr) -> Result<(), String> {
    let mut stream = TcpStream::connect(addr).await.map_err(|e| e.to_string())?;
    let _sent = stream.write_all(HEALTH).await;
    let mut chunk = [0_u8; 256];
    match timeout_real(stream.read(&mut chunk), WAIT).await? {
        Ok(0) | Err(_) => Ok(()),
        Ok(_) => Err("the connection over the limit got an answer".to_owned()),
    }
}

async fn limit_then_unlimited(unlimited: Option<IpAddr>) -> Result<(), String> {
    let public: IpAddr = "1.2.3.4".parse().map_err(|_e| "bad ip".to_owned())?;
    let [public_addr, other_addr] = start([Some(public), unlimited]).await?;
    let held_public = open_many(public_addr, PUBLIC_PEER_CONNECTIONS).await?;
    refused(public_addr).await?;
    let held_other = open_many(other_addr, 100).await?;
    assert_eq!(held_other.len(), 100);
    refused(public_addr).await?;
    drop(held_public);
    Ok(())
}

#[tokio::test]
async fn t_safety_n10_4() -> Result<(), String> {
    limit_then_unlimited(None).await
}

#[tokio::test]
async fn t_server_n2_3() -> Result<(), String> {
    let lan: IpAddr = "192.168.1.20".parse().map_err(|_e| "bad ip".to_owned())?;
    limit_then_unlimited(Some(lan)).await
}

#[tokio::test]
async fn a_closed_public_connection_frees_its_place() -> Result<(), String> {
    let public: IpAddr = "1.2.3.4".parse().map_err(|_e| "bad ip".to_owned())?;
    let [public_addr, _other] = start([Some(public), None]).await?;
    let mut held = open_many(public_addr, PUBLIC_PEER_CONNECTIONS).await?;
    refused(public_addr).await?;
    let closed = held.pop().ok_or("no connection to close".to_owned())?;
    drop(closed);
    for _attempt in 0..50 {
        if open_and_ask(public_addr).await.is_ok() {
            return Ok(());
        }
        test_harness::sleep_real(Duration::from_millis(100)).await;
    }
    Err("the freed place never came back".to_owned())
}
