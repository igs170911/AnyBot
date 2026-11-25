use anybot_blob as _;
use anybot_bus as _;
use anybot_channels as _;
use anybot_domain as _;
use anybot_foundation as _;
use anybot_protocol as _;
use anybot_store_server as _;
use axum as _;
use getrandom as _;
use serde as _;
use serde_json as _;
use socket2 as _;
use std::net::SocketAddr;
use test_harness as _;
use time as _;
use utoipa as _;

use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};

use anybot_server::serve_all::{CODE_SERVER_STOP_NO_LISTENER, serve_all};
use anybot_server::{AppState, build_router};

async fn bind_pair() -> Result<(TcpListener, TcpListener, SocketAddr, SocketAddr), String> {
    let v4 = SocketAddr::from(([127, 0, 0, 1], 0));
    let v6 = SocketAddr::from((std::net::Ipv6Addr::LOCALHOST, 0));
    let first = TcpListener::bind(v4).await.map_err(|e| e.to_string())?;
    let second = TcpListener::bind(v6).await.map_err(|e| e.to_string())?;
    let a4 = first.local_addr().map_err(|e| e.to_string())?;
    let a6 = second.local_addr().map_err(|e| e.to_string())?;
    Ok((first, second, a4, a6))
}

async fn get_status(addr: SocketAddr, path: &str) -> Result<String, String> {
    let stream = TcpStream::connect(addr).await.map_err(|e| e.to_string())?;
    let (mut reader, mut writer) = stream.into_split();
    let request = format!("GET {path} HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n");
    writer
        .write_all(request.as_bytes())
        .await
        .map_err(|e| e.to_string())?;
    let mut raw = Vec::new();
    reader
        .read_to_end(&mut raw)
        .await
        .map_err(|e| e.to_string())?;
    String::from_utf8(raw).map_err(|e| e.to_string())
}

#[tokio::test]
async fn serve_all_serves_every_address_at_the_same_time() {
    let (first, second, a4, a6) = bind_pair().await.unwrap();
    let app = build_router(AppState::new("test-9.9.9"));
    let handle = tokio::spawn(async move { serve_all(vec![first, second], app).await });
    let head4 = get_status(a4, "/v1/health").await.unwrap();
    let head6 = get_status(a6, "/v1/health").await.unwrap();
    assert!(head4.contains("HTTP/1.1 200 OK"), "{head4}");
    assert!(head6.contains("HTTP/1.1 200 OK"), "{head6}");
    handle.abort();
}

#[tokio::test]
async fn serve_all_reports_when_no_address_is_served() {
    let app = build_router(AppState::new("test-9.9.9"));
    let reason = match serve_all(Vec::new(), app).await {
        Ok(reason) | Err(reason) => reason,
    };
    assert_eq!(reason.code, CODE_SERVER_STOP_NO_LISTENER);
}
