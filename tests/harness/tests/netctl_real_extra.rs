use aes_gcm as _;
use anybot_foundation as _;
use anybot_store_server as _;
use axum as _;
use base64 as _;
use bytes as _;
use getrandom as _;
use hkdf as _;
use libc as _;
use p256 as _;
use postgresql_embedded as _;
use rcgen as _;
use rustls as _;
use rustls_pki_types as _;
use sad_web_push as _;
use serde as _;
use serde_json as _;
use sha2 as _;
use socket2 as _;
use sqlx as _;
use time as _;
use tokio_rustls as _;
use tokio_stream as _;
use toml as _;
use tracing as _;
use tracing_subscriber as _;
use url as _;

use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;

use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;

use reqwest as _;
use test_harness::fake_clock::FakeClock;
use test_harness::netctl::{
    net_attach, net_buffer, net_drop_response, net_idle_cut_clock, net_stall, net_start,
};
use test_harness::realtime::timeout_real;
use test_harness::{
    NetCtl, advance_clock, first_data_delay, get_via_proxy, origin_url, start_origin,
};

const BUDGET: Duration = Duration::from_secs(30);

async fn start_env(origin: SocketAddr) -> Result<(Arc<NetCtl>, u16), String> {
    let (ctl, proxy_port) = timeout_real(net_start(), BUDGET).await??;
    net_attach(&ctl, proxy_port, origin).await?;
    Ok((ctl, proxy_port))
}

async fn raw_connect(proxy_port: u16) -> Result<TcpStream, String> {
    let connect = timeout_real(TcpStream::connect(("127.0.0.1", proxy_port)), BUDGET).await;
    match connect {
        Ok(Ok(stream)) => Ok(stream),
        Ok(Err(e)) => Err(e.to_string()),
        Err(_e) => Err("connect to proxy timed out".to_owned()),
    }
}

async fn raw_get(proxy_port: u16, origin: SocketAddr, route: &str) -> Result<TcpStream, String> {
    let mut client = raw_connect(proxy_port).await?;
    let request = format!(
        "GET {}{} HTTP/1.1\r\nHost: origin\r\n\r\n",
        origin_url(origin, ""),
        route
    );
    timeout_real(client.write_all(request.as_bytes()), BUDGET)
        .await?
        .map_err(|e| e.to_string())?;
    Ok(client)
}

async fn read_window(client: &mut TcpStream, window: Duration) -> Result<Vec<u8>, String> {
    let mut got: Vec<u8> = Vec::new();
    let mut byte = [0u8; 1];
    loop {
        let read = timeout_real(client.read(&mut byte), window).await;
        let Ok(Ok(count)) = read else {
            return Ok(got);
        };
        if count == 0 {
            return Ok(got);
        }
        let Some(b) = byte.first() else {
            return Ok(got);
        };
        got.push(*b);
    }
}

#[tokio::test]
async fn netctl_real_buffer_streams_when_upstream_marks_no_buffering() {
    let origin = start_origin().await.unwrap();
    let (ctl, proxy_port) = start_env(origin).await.unwrap();
    net_buffer(&ctl, proxy_port, true).await.unwrap();
    let streamed = first_data_delay(proxy_port, origin, "/nobuffer")
        .await
        .unwrap();
    assert_eq!(streamed.1, "first-half-line!\nsecond-half-line!\n");
    assert!(
        streamed.0 < Duration::from_millis(400),
        "X-Accel-Buffering: no must stream even in buffer mode: {:?}",
        streamed.0
    );
    let held = first_data_delay(proxy_port, origin, "/stall")
        .await
        .unwrap();
    assert!(
        held.0 >= Duration::from_millis(450),
        "plain responses stay buffered: {:?}",
        held.0
    );
}

#[tokio::test]
async fn netctl_real_stall_keeps_the_connection_open_and_swallows_both_ways() {
    let origin = start_origin().await.unwrap();
    let (ctl, proxy_port) = start_env(origin).await.unwrap();
    net_stall(&ctl, proxy_port).await.unwrap();
    let mut client = raw_get(proxy_port, origin, "/hello").await.unwrap();
    let got = read_window(&mut client, Duration::from_millis(400))
        .await
        .unwrap();
    assert_eq!(got.len(), 0, "stall must swallow the response: {got:?}");
    let peer = read_window(&mut client, Duration::from_millis(50))
        .await
        .unwrap();
    assert_eq!(peer.len(), 0, "stall connection must stay open, no EOF");
}

#[tokio::test]
async fn netctl_real_drop_response_reaches_server_then_closes() {
    let origin = start_origin().await.unwrap();
    let (ctl, proxy_port) = start_env(origin).await.unwrap();
    net_drop_response(&ctl, proxy_port, true).await.unwrap();
    let first = get_via_proxy(proxy_port, origin, "/hits").await;
    assert!(first.is_err(), "dropped response must fail the client");
    let second = get_via_proxy(proxy_port, origin, "/hits").await;
    assert!(
        second.is_err(),
        "dropped response must fail the client again"
    );
    let mut direct = TcpStream::connect(origin).await.unwrap();
    let request = format!(
        "GET {} HTTP/1.1\r\nHost: origin\r\n\r\n",
        origin_url(origin, "/hits")
    );
    direct.write_all(request.as_bytes()).await.unwrap();
    let mut body = Vec::new();
    direct.read_to_end(&mut body).await.unwrap();
    let text = String::from_utf8_lossy(&body).into_owned();
    assert!(
        text.contains('3'),
        "each dropped request still reached the origin: {text}"
    );
}

#[tokio::test]
async fn netctl_real_drop_response_open_leaves_the_client_waiting() {
    let origin = start_origin().await.unwrap();
    let (ctl, proxy_port) = start_env(origin).await.unwrap();
    net_drop_response(&ctl, proxy_port, false).await.unwrap();
    let mut client = raw_get(proxy_port, origin, "/hello").await.unwrap();
    let started = std::time::Instant::now();
    let got = read_window(&mut client, Duration::from_secs(2))
        .await
        .unwrap();
    assert_eq!(got.len(), 0, "open drop must swallow and stay silent");
    assert!(
        started.elapsed() >= Duration::from_secs(1),
        "client keeps waiting; the connection stays open"
    );
}

#[tokio::test]
async fn netctl_real_idle_cut_clock_stays_quiet_until_the_clock_arrives() {
    let origin = start_origin().await.unwrap();
    let (ctl, proxy_port) = start_env(origin).await.unwrap();
    let clock = FakeClock::new(0);
    net_idle_cut_clock(&ctl, proxy_port, Duration::from_millis(600), &clock)
        .await
        .unwrap();
    let mut client = raw_get(proxy_port, origin, "/quiet").await.unwrap();
    let early = read_window(&mut client, Duration::from_millis(300))
        .await
        .unwrap();
    let early = String::from_utf8_lossy(&early).into_owned();
    assert!(early.contains("tick"), "first data arrives: {early}");
    advance_clock(&clock, Duration::from_millis(500));
    let quiet = read_window(&mut client, Duration::from_millis(300))
        .await
        .unwrap();
    let quiet = String::from_utf8_lossy(&quiet).into_owned();
    assert!(
        !quiet.contains("Idle Timeout"),
        "fake clock must not cut before the idle span: {quiet}"
    );
}

#[tokio::test]
async fn netctl_real_idle_cut_clock_cuts_when_the_clock_passes_idle() {
    let origin = start_origin().await.unwrap();
    let (ctl, proxy_port) = start_env(origin).await.unwrap();
    let clock = FakeClock::new(0);
    net_idle_cut_clock(&ctl, proxy_port, Duration::from_millis(600), &clock)
        .await
        .unwrap();
    let mut client = raw_get(proxy_port, origin, "/quiet").await.unwrap();
    let early = read_window(&mut client, Duration::from_millis(300))
        .await
        .unwrap();
    let early = String::from_utf8_lossy(&early).into_owned();
    assert!(early.contains("tick"), "first data arrives: {early}");
    advance_clock(&clock, Duration::from_millis(650));
    let cut = read_window(&mut client, Duration::from_millis(500))
        .await
        .unwrap();
    let cut = String::from_utf8_lossy(&cut).into_owned();
    assert!(
        cut.contains("Idle Timeout"),
        "passing the idle span on the fake clock must cut: {cut}"
    );
}

#[tokio::test]
async fn netctl_real_idle_cut_clock_data_resets_the_idle_timer() {
    let origin = start_origin().await.unwrap();
    let (ctl, proxy_port) = start_env(origin).await.unwrap();
    let clock = FakeClock::new(0);
    net_idle_cut_clock(&ctl, proxy_port, Duration::from_millis(600), &clock)
        .await
        .unwrap();
    let mut client = raw_get(proxy_port, origin, "/beat").await.unwrap();
    let mut collected: Vec<u8> = Vec::new();
    for _round in 0..15 {
        let piece = read_window(&mut client, Duration::from_millis(300))
            .await
            .unwrap();
        collected.extend_from_slice(&piece);
        advance_clock(&clock, Duration::from_millis(400));
    }
    let text = String::from_utf8_lossy(&collected).into_owned();
    assert_eq!(text.len(), 175, "full beat stream arrives: {text}");
    assert!(
        !text.contains("Idle Timeout"),
        "each gap was 400 on the fake clock, below the 600 idle: {text}"
    );
}
