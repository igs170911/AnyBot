use anybot_app_server as _;
use anybot_domain as _;
use anybot_foundation as _;
use anybot_messages as _;
use anybot_server as _;
use anybot_server_test as _;
use anybot_store_server as _;
use axum as _;
use clap as _;
use rpassword as _;
use sqlx as _;
use tracing as _;

use std::time::Duration;

use serde_json::{Value, json};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;

use anybot_app_test::http::{LOGIN_STEP, RequestSpec, TEST_CLIENT_VERSION, post_driven};
use anybot_app_test::serve::{Served, boot_pair, serve_start};
use test_harness::timeout_real;

const TOO_LARGE: &str = "server-request-too-large";
const KB: u64 = 1024;
const MB: u64 = 1024 * 1024;

struct Answer {
    status: u16,
    code: Option<String>,
}

async fn bearer_of(served: &Served) -> Result<String, String> {
    let body = json!({"account": "bob", "password": "bobpassword", "label": "uploads", "credential": "bearer"});
    let spec = RequestSpec::new(&body.to_string()).with_key("k-login-uploads");
    let answer = post_driven(
        &served.clock,
        &served.base_url(),
        "/v1/account/login",
        &spec,
        LOGIN_STEP,
        Duration::from_secs(15),
    )
    .await?;
    answer.bearer()
}

fn header_end(buf: &[u8]) -> Option<usize> {
    buf.windows(4)
        .position(|w| w == b"\r\n\r\n")
        .map(|p| p.saturating_add(4))
}

fn parse_answer(raw: &[u8]) -> Result<Answer, String> {
    let end = header_end(raw).ok_or("no header end".to_owned())?;
    let head = String::from_utf8_lossy(raw.get(..end).unwrap_or_default()).into_owned();
    let status = head
        .split_whitespace()
        .nth(1)
        .and_then(|text| text.parse().ok())
        .ok_or(format!("no status in {head}"))?;
    let body: Value = serde_json::from_slice(raw.get(end..).unwrap_or_default())
        .map_err(|e| format!("answer body is not JSON: {e}"))?;
    let code = body.get("code").and_then(Value::as_str).map(str::to_owned);
    Ok(Answer { status, code })
}

async fn read_until_closed(stream: &mut TcpStream) -> Result<Vec<u8>, String> {
    let mut raw = Vec::new();
    stream
        .read_to_end(&mut raw)
        .await
        .map_err(|e| e.to_string())?;
    Ok(raw)
}

async fn declare_only(
    served: &Served,
    path: &str,
    bearer: Option<&str>,
    length: u64,
) -> Result<Answer, String> {
    let auth = bearer.map_or(String::new(), |key| {
        format!("Authorization: Bearer {key}\r\n")
    });
    let head = format!(
        "POST {path} HTTP/1.1\r\nHost: 127.0.0.1\r\nContent-Type: application/json\r\nanybot-client-version: {TEST_CLIENT_VERSION}\r\nIdempotency-Key: k-size-{length}\r\n{auth}Content-Length: {length}\r\nConnection: close\r\n\r\n"
    );
    let mut stream = TcpStream::connect(served.origin_addr())
        .await
        .map_err(|e| e.to_string())?;
    stream
        .write_all(head.as_bytes())
        .await
        .map_err(|e| e.to_string())?;
    let raw = timeout_real(read_until_closed(&mut stream), Duration::from_secs(10)).await??;
    parse_answer(&raw)
}

#[tokio::test]
async fn bodies_over_the_sign_in_cap_are_refused_before_reading() -> Result<(), String> {
    let served = serve_start("upload_caps").await?;
    boot_pair(&served).await?;
    let guest = declare_only(&served, "/v1/account/login", None, 257 * KB).await?;
    assert_eq!(guest.status, 413);
    assert_eq!(guest.code.as_deref(), Some(TOO_LARGE));
    let bearer = bearer_of(&served).await?;
    let signed = declare_only(&served, "/v1/streams/append", Some(&bearer), 65 * MB).await?;
    assert_eq!(signed.status, 413);
    assert_eq!(signed.code.as_deref(), Some(TOO_LARGE));
    let forged = declare_only(&served, "/v1/streams/append", Some("not-a-key"), 257 * KB).await?;
    assert_eq!(forged.status, 413, "an unknown key is not signed in");
    served.stop().await
}
