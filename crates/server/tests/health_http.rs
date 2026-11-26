use anybot_blob as _;
use anybot_bus as _;
use anybot_channels as _;
use anybot_domain as _;
use anybot_foundation as _;
use anybot_messages as _;
use anybot_protocol as _;
use anybot_store_server as _;
use argon2 as _;
use getrandom as _;
use http_body_util as _;
use serde as _;
use sha2 as _;
use socket2 as _;
use sqlx as _;
use std::net::TcpListener as StdTcpListener;
use test_harness as _;
use time as _;
use url as _;
use utoipa as _;
use uuid as _;

use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};

use anybot_server::{AppState, build_router};

fn start_test_server() -> Result<u16, String> {
    let std_listener = StdTcpListener::bind("127.0.0.1:0").map_err(|e| e.to_string())?;
    let port = std_listener.local_addr().map_err(|e| e.to_string())?.port();
    std_listener
        .set_nonblocking(true)
        .map_err(|e| e.to_string())?;
    let listener = TcpListener::from_std(std_listener).map_err(|e| e.to_string())?;
    let app = build_router(AppState::new("test-9.9.9"));
    tokio::spawn(async move {
        match axum::serve(listener, app).await {
            Ok(()) => {}
            Err(err) => {
                let mut stderr = std::io::stderr();
                let _result = std::io::Write::write_fmt(
                    &mut stderr,
                    format_args!("test server stopped{err}\n"),
                );
            }
        }
    });
    Ok(port)
}

async fn get_response(port: u16, path: &str) -> Result<(String, String), String> {
    let stream = TcpStream::connect(("127.0.0.1", port))
        .await
        .map_err(|e| e.to_string())?;
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
    let text = String::from_utf8(raw).map_err(|e| e.to_string())?;
    let (head, body) = text
        .split_once("\r\n\r\n")
        .ok_or("response lacks a chunk separator".to_owned())?;
    Ok((head.to_owned(), body.to_owned()))
}

#[tokio::test]
async fn test_health_ok_json_matches_schema_fields() {
    let port = start_test_server().unwrap();
    let (head, body) = get_response(port, "/v1/health").await.unwrap();
    assert!(head.contains("HTTP/1.1 200 OK"));
    assert!(head.contains("content-type: application/json"));
    let value: serde_json::Value = serde_json::from_str(&body).unwrap();
    assert_eq!(
        value.get("version").and_then(|v| v.as_str()),
        Some("test-9.9.9")
    );
    assert_eq!(
        value
            .get("protocol_version")
            .and_then(serde_json::Value::as_u64),
        Some(1)
    );
    let now = value.get("now").and_then(|v| v.as_str()).unwrap();
    assert!(now.contains('T') && now.ends_with('Z'));
}

#[tokio::test]
async fn test_health_unknown_path_is_not_found() {
    let port = start_test_server().unwrap();
    let (head, _body) = get_response(port, "/v1/nosuch").await.unwrap();
    assert!(head.contains("HTTP/1.1 404"));
}
