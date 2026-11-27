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
use test_harness::netctl::{NetMode, net_attach, net_mode, net_start};
use test_harness::netctl_modes::{net_drop_events, net_throttle};
use test_harness::realtime::timeout_real;
use test_harness::{NetCtl, advance_clock, get_via_proxy, origin_url, start_origin};

const BUDGET: Duration = Duration::from_secs(30);
const HEAD_LEN: usize = "HTTP/1.1 200 OK\r\nContent-Length: 100000\r\n\r\n".len();

async fn start_env(origin: SocketAddr) -> Result<(Arc<NetCtl>, u16), String> {
    let (ctl, proxy_port) = timeout_real(net_start(), BUDGET).await??;
    net_attach(&ctl, proxy_port, origin).await?;
    Ok((ctl, proxy_port))
}

async fn raw_get(proxy_port: u16, origin: SocketAddr, route: &str) -> Result<TcpStream, String> {
    let mut client = timeout_real(TcpStream::connect(("127.0.0.1", proxy_port)), BUDGET)
        .await?
        .map_err(|e| e.to_string())?;
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

async fn read_until_quiet(client: &mut TcpStream, quiet: Duration) -> usize {
    let mut total = 0usize;
    let mut buf = vec![0u8; 16 * 1024];
    loop {
        let read = timeout_real(client.read(&mut buf), quiet).await;
        match read {
            Ok(Ok(count)) if count > 0 => total = total.saturating_add(count),
            _ended => return total,
        }
    }
}

#[tokio::test]
async fn netctl_throttle_gives_no_bytes_while_the_fake_clock_stands_still() {
    let origin = start_origin().await.unwrap();
    let (ctl, proxy_port) = start_env(origin).await.unwrap();
    let clock = FakeClock::new(1_000_000);
    net_throttle(&ctl, proxy_port, 2_000, &clock).await.unwrap();
    assert_eq!(
        net_mode(&ctl, proxy_port).await,
        NetMode::Throttle {
            bytes_per_sec: 2_000
        }
    );
    let mut client = raw_get(proxy_port, origin, "/bulk").await.unwrap();
    let early = read_until_quiet(&mut client, Duration::from_millis(600)).await;
    assert_eq!(early, 0, "real time passing must not release any bytes");
}

#[tokio::test]
async fn netctl_throttle_releases_the_rate_per_fake_second_in_20_second_steps() {
    let origin = start_origin().await.unwrap();
    let (ctl, proxy_port) = start_env(origin).await.unwrap();
    let clock = FakeClock::new(1_000_000);
    net_throttle(&ctl, proxy_port, 2_000, &clock).await.unwrap();
    let mut client = raw_get(proxy_port, origin, "/bulk").await.unwrap();
    let early = read_until_quiet(&mut client, Duration::from_millis(400)).await;
    assert_eq!(early, 0);
    let started = std::time::Instant::now();
    let _woken = advance_clock(&clock, Duration::from_secs(20));
    let first = read_until_quiet(&mut client, Duration::from_millis(400)).await;
    assert_eq!(first, 40_000);
    let _woken = advance_clock(&clock, Duration::from_secs(20));
    let second = read_until_quiet(&mut client, Duration::from_millis(400)).await;
    assert_eq!(second, 40_000);
    let _woken = advance_clock(&clock, Duration::from_millis(500));
    let third = read_until_quiet(&mut client, Duration::from_millis(400)).await;
    assert_eq!(third, 1_000);
    let _woken = advance_clock(&clock, Duration::from_secs(20));
    let rest = read_until_quiet(&mut client, Duration::from_millis(400)).await;
    assert_eq!(rest, HEAD_LEN + 100_000 - 81_000);
    assert!(started.elapsed() < Duration::from_secs(10));
}

fn tick_numbers(body: &str) -> Result<Vec<u32>, String> {
    let mut ticks = Vec::new();
    for piece in body.split("\n\n").filter(|p| !p.is_empty()) {
        if piece == ":hb" {
            continue;
        }
        let Some(number) = piece.strip_prefix("event: tick\ndata: ") else {
            return Err(format!("half or foreign event on the wire: {piece:?}"));
        };
        ticks.push(number.parse::<u32>().map_err(|e| e.to_string())?);
    }
    Ok(ticks)
}

fn heartbeats(body: &str) -> usize {
    body.split("\n\n").filter(|p| *p == ":hb").count()
}

async fn events_body(proxy_port: u16, origin: SocketAddr) -> Result<String, String> {
    let (status, body) = get_via_proxy(proxy_port, origin, "/events").await?;
    if status != 200 {
        return Err(format!("status {status}"));
    }
    Ok(body)
}

#[tokio::test]
async fn netctl_drop_events_drops_about_the_given_ratio_and_only_whole_events() {
    let origin = start_origin().await.unwrap();
    let (ctl, proxy_port) = start_env(origin).await.unwrap();
    net_drop_events(&ctl, proxy_port, 300, 42).await.unwrap();
    let body = events_body(proxy_port, origin).await.unwrap();
    let ticks = tick_numbers(&body).unwrap();
    assert!(
        (620..=780).contains(&ticks.len()),
        "kept {} of 1000 at 300 per mille dropped",
        ticks.len()
    );
    assert!(ticks.is_sorted_by(|a, b| a < b));
    assert_eq!(heartbeats(&body), 10, "comments are never dropped");
}

#[tokio::test]
async fn netctl_drop_events_is_reproducible_with_the_seed_and_differs_between_seeds() {
    let origin = start_origin().await.unwrap();
    let (ctl, proxy_port) = start_env(origin).await.unwrap();
    net_drop_events(&ctl, proxy_port, 300, 42).await.unwrap();
    let first = events_body(proxy_port, origin).await.unwrap();
    net_drop_events(&ctl, proxy_port, 300, 42).await.unwrap();
    let again = events_body(proxy_port, origin).await.unwrap();
    assert_eq!(first, again);
    net_drop_events(&ctl, proxy_port, 300, 43).await.unwrap();
    let other = events_body(proxy_port, origin).await.unwrap();
    assert_ne!(first, other);
}

#[tokio::test]
async fn netctl_drop_events_edges_keep_all_or_drop_all_events() {
    let origin = start_origin().await.unwrap();
    let (ctl, proxy_port) = start_env(origin).await.unwrap();
    net_drop_events(&ctl, proxy_port, 0, 5).await.unwrap();
    let all = events_body(proxy_port, origin).await.unwrap();
    assert_eq!(tick_numbers(&all).unwrap().len(), 1_000);
    net_drop_events(&ctl, proxy_port, 1_000, 5).await.unwrap();
    let none = events_body(proxy_port, origin).await.unwrap();
    assert_eq!(tick_numbers(&none).unwrap().len(), 0);
    assert_eq!(heartbeats(&none), 10);
}

#[tokio::test]
async fn netctl_drop_events_leaves_other_responses_untouched() {
    let origin = start_origin().await.unwrap();
    let (ctl, proxy_port) = start_env(origin).await.unwrap();
    net_drop_events(&ctl, proxy_port, 1_000, 5).await.unwrap();
    let hello = get_via_proxy(proxy_port, origin, "/hello").await.unwrap();
    assert_eq!(hello, (200, "hello-origin".to_owned()));
}

#[tokio::test]
async fn netctl_proxy_serves_a_new_connection_while_another_one_stays_open() {
    let origin = start_origin().await.unwrap();
    let (ctl, proxy_port) = start_env(origin).await.unwrap();
    let clock = FakeClock::new(1_000_000);
    net_throttle(&ctl, proxy_port, 1_000, &clock).await.unwrap();
    let mut held = raw_get(proxy_port, origin, "/bulk").await.unwrap();
    let early = read_until_quiet(&mut held, Duration::from_millis(300)).await;
    assert_eq!(early, 0);
    test_harness::net_restore(&ctl, proxy_port).await.unwrap();
    let fresh = get_via_proxy(proxy_port, origin, "/hello").await.unwrap();
    assert_eq!(fresh, (200, "hello-origin".to_owned()));
}
