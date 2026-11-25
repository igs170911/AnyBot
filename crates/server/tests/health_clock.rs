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
use serde as _;
use sha2 as _;
use socket2 as _;
use sqlx as _;
use url as _;
use utoipa as _;
use uuid as _;

use std::net::TcpListener as StdTcpListener;

use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};

use anybot_foundation::clock::{Clock, TimestampMs};
use anybot_server::{AppState, build_router};

use test_harness::fake_clock::FakeClock;

use std::sync::Arc;
use std::time::Duration;

fn start_clocked_server(clock: Arc<FakeClock>) -> Result<u16, String> {
    let std_listener = StdTcpListener::bind("127.0.0.1:0").map_err(|e| e.to_string())?;
    let port = std_listener.local_addr().map_err(|e| e.to_string())?.port();
    std_listener
        .set_nonblocking(true)
        .map_err(|e| e.to_string())?;
    let listener = TcpListener::from_std(std_listener).map_err(|e| e.to_string())?;
    let app = build_router(AppState::with_clock("test-9.9.9", clock));
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

async fn get_health_body(port: u16) -> Result<serde_json::Value, String> {
    let stream = TcpStream::connect(("127.0.0.1", port))
        .await
        .map_err(|e| e.to_string())?;
    let (mut reader, mut writer) = stream.into_split();
    let request = "GET /v1/health HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n";
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
    let (_head, body) = text
        .split_once("\r\n\r\n")
        .ok_or("response lacks a body separator".to_owned())?;
    serde_json::from_str(body).map_err(|e| e.to_string())
}

fn now_field_ms(body: &serde_json::Value) -> Result<u64, String> {
    let Some(now) = body.get("now").and_then(|v| v.as_str()) else {
        return Err("health body has no now field".to_owned());
    };
    let parsed = time::OffsetDateTime::parse(now, &time::format_description::well_known::Rfc3339)
        .map_err(|e| e.to_string())?;
    let secs_ms = parsed
        .unix_timestamp()
        .checked_mul(1_000)
        .map(u64::try_from)
        .ok_or("health now seconds overflow".to_owned())?
        .map_err(|e| e.to_string())?;
    let micro_ms = u64::from(parsed.microsecond() / 1_000);
    secs_ms
        .checked_add(micro_ms)
        .ok_or("health now millis overflow".to_owned())
}

#[tokio::test]
async fn fake_clock_advance_moves_health_now_by_one_hour() {
    let start_ms: u64 = 1_700_000_000_000;
    let clock = FakeClock::new(start_ms);
    let port = start_clocked_server(Arc::clone(&clock)).unwrap();
    let first = get_health_body(port).await.unwrap();
    let first_ms = now_field_ms(&first).unwrap();
    assert_eq!(first_ms, start_ms);
    let _woken = clock.advance(Duration::from_secs(3_600));
    let second = get_health_body(port).await.unwrap();
    let second_ms = now_field_ms(&second).unwrap();
    assert_eq!(second_ms - first_ms, 3_600_000);
    assert_eq!(clock.now_ms(), start_ms + 3_600_000);
    let as_clock: &dyn Clock = &*clock;
    assert_eq!(
        as_clock.now_ms(),
        TimestampMs::from_millis(start_ms + 3_600_000)
    );
}
