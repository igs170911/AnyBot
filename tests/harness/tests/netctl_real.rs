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
use reqwest as _;
use sad_web_push as _;
use serde as _;
use serde_json as _;
use sha2 as _;
use socket2 as _;
use sqlx as _;
use tokio_stream as _;
use toml as _;
use tracing as _;
use tracing_subscriber as _;
use url as _;

use std::net::{IpAddr, SocketAddr};
use std::sync::Arc;
use std::time::{Duration, Instant};

use tokio::io::AsyncReadExt;
use tokio::io::AsyncWriteExt;
use tokio::net::TcpStream;

use test_harness::http::client_via_proxy;
use test_harness::netctl::{
    NetMode, net_attach, net_buffer, net_cut, net_down, net_idle_cut, net_mode, net_restore,
    net_slow, net_source_addr, net_start,
};
use test_harness::realtime::timeout_real;
use test_harness::{
    NetCtl, collect_stream, first_data_delay, get_via_proxy, origin_url, start_origin,
    start_origin_v6,
};

const BUDGET: Duration = Duration::from_secs(30);
const LOOPBACK_V6: IpAddr = IpAddr::V6(std::net::Ipv6Addr::LOCALHOST);

async fn start_env(origin: SocketAddr) -> Result<(Arc<NetCtl>, u16), String> {
    let (ctl, proxy_port) = timeout_real(net_start(), BUDGET).await??;
    timeout_real(net_attach(&ctl, proxy_port, origin), BUDGET)
        .await?
        .map_err(|e| format!("attach failed: {e}"))?;
    Ok((ctl, proxy_port))
}

async fn connect_refused_retry(proxy_port: u16) -> Result<TcpStream, std::io::Error> {
    let deadline = Instant::now()
        .checked_add(Duration::from_secs(5))
        .ok_or(std::io::Error::other("deadline overflow"))?;
    loop {
        let stream = TcpStream::connect(("127.0.0.1", proxy_port)).await?;
        if Instant::now() >= deadline {
            return Ok(stream);
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
}

async fn get_via_proxy_retry(
    proxy_port: u16,
    origin: SocketAddr,
    route: &str,
) -> Result<(u16, String), String> {
    let deadline = Instant::now()
        .checked_add(Duration::from_secs(10))
        .ok_or("deadline overflow".to_owned())?;
    loop {
        let attempt = get_via_proxy(proxy_port, origin, route).await;
        if attempt.is_ok() {
            return attempt;
        }
        if Instant::now() >= deadline {
            return attempt;
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
}

#[tokio::test]
async fn netctl_real_cut_severs_live_and_blocks_new_requests() {
    let origin = start_origin().await.unwrap();
    let (ctl, proxy_port) = start_env(origin).await.unwrap();
    let client = client_via_proxy(&format!("http://127.0.0.1:{proxy_port}")).unwrap();
    let response = client
        .get(origin_url(origin, "/beat"))
        .send()
        .await
        .unwrap();
    tokio::time::sleep(Duration::from_millis(250)).await;
    net_cut(&ctl, proxy_port).await.unwrap();
    let rest = timeout_real(collect_stream(response), BUDGET)
        .await
        .unwrap_or_else(|_e| String::from("stream hung"));
    assert!(rest.len() < 135, "live stream must be severed: {rest}");
    let blocked = test_harness::http_send_get(&client, &origin_url(origin, "/hello")).await;
    assert!(blocked.is_err(), "new request during cut must fail");
    net_restore(&ctl, proxy_port).await.unwrap();
    let reopened = get_via_proxy_retry(proxy_port, origin, "/hello")
        .await
        .unwrap();
    assert_eq!(reopened.0, 200);
}

#[tokio::test]
async fn netctl_real_slow_latency_and_bandwidth_are_measurable() {
    let origin = start_origin().await.unwrap();
    let (ctl, proxy_port) = start_env(origin).await.unwrap();
    net_slow(&ctl, proxy_port, Duration::from_millis(200))
        .await
        .unwrap();
    let started = Instant::now();
    let hello = get_via_proxy(proxy_port, origin, "/hello").await.unwrap();
    assert_eq!(hello.0, 200);
    assert!(
        started.elapsed() >= Duration::from_millis(150),
        "slow connect latency must be measurable"
    );
    net_slow(&ctl, proxy_port, Duration::from_millis(10))
        .await
        .unwrap();
    let started = Instant::now();
    let paced = get_via_proxy(proxy_port, origin, "/gap").await.unwrap();
    let elapsed = started.elapsed();
    assert_eq!(paced.1.len(), 60);
    assert!(
        elapsed >= Duration::from_millis(400),
        "bandwidth limit must be measurable: {elapsed:?}"
    );
}

#[tokio::test]
async fn netctl_real_down_refuses_connections_and_restore_reopens() {
    let origin = start_origin().await.unwrap();
    let (ctl, proxy_port) = start_env(origin).await.unwrap();
    net_down(&ctl, proxy_port).await.unwrap();
    let refused = connect_refused_retry(proxy_port).await;
    assert!(refused.is_err(), "down server must refuse TCP connections");
    let blocked = get_via_proxy(proxy_port, origin, "/hello").await;
    assert!(blocked.is_err(), "down server must block HTTP");
    net_restore(&ctl, proxy_port).await.unwrap();
    let reopened = get_via_proxy_retry(proxy_port, origin, "/hello")
        .await
        .unwrap();
    assert_eq!(reopened, (200, "hello-origin".to_owned()));
}

#[tokio::test]
async fn netctl_real_buffer_holds_the_stream_until_close() {
    let origin = start_origin().await.unwrap();
    let (ctl, proxy_port) = start_env(origin).await.unwrap();
    let open = first_data_delay(proxy_port, origin, "/stall")
        .await
        .unwrap();
    assert!(
        open.0 < Duration::from_millis(400),
        "open mode streams early: {:?}",
        open.0
    );
    net_buffer(&ctl, proxy_port, true).await.unwrap();
    let buffered = first_data_delay(proxy_port, origin, "/stall")
        .await
        .unwrap();
    assert_eq!(buffered.1, "first-half-line!\nsecond-half-line!\n");
    assert!(
        buffered.0 >= Duration::from_millis(450),
        "buffered proxy must deliver only after the upstream closes: {:?}",
        buffered.0
    );
}

#[tokio::test]
async fn netctl_real_idle_cut_closes_silent_streams_only() {
    let origin = start_origin().await.unwrap();
    let (ctl, proxy_port) = start_env(origin).await.unwrap();
    net_idle_cut(&ctl, proxy_port, Duration::from_millis(600))
        .await
        .unwrap();
    let busy = get_via_proxy(proxy_port, origin, "/beat").await.unwrap();
    assert_eq!(busy.1.len(), 135);
    assert!(!busy.1.contains("Idle Timeout"));
    let started = Instant::now();
    let raw = raw_via_proxy(proxy_port, origin, "/quiet").await.unwrap();
    assert!(
        raw.contains("Idle Timeout"),
        "silent stream must be cut with a marker: {raw}"
    );
    assert!(
        started.elapsed() < Duration::from_millis(1500),
        "idle cut must fire well before the 2s silence ends"
    );
}

async fn raw_via_proxy(proxy_port: u16, origin: SocketAddr, route: &str) -> Result<String, String> {
    let connect = timeout_real(TcpStream::connect(("127.0.0.1", proxy_port)), BUDGET).await;
    let mut client = match connect {
        Ok(Ok(stream)) => stream,
        Ok(Err(e)) => return Err(e.to_string()),
        Err(_e) => return Err("connect to proxy timed out".to_owned()),
    };
    let request = format!(
        "GET {}{} HTTP/1.1\r\nHost: origin\r\n\r\n",
        origin_url(origin, ""),
        route
    );
    timeout_real(client.write_all(request.as_bytes()), BUDGET)
        .await?
        .map_err(|e| e.to_string())?;
    let mut raw = Vec::new();
    let mut byte = [0u8; 1];
    loop {
        let read = timeout_real(client.read(&mut byte), Duration::from_secs(3)).await;
        let Ok(Ok(count)) = read else {
            break;
        };
        if count == 0 {
            break;
        }
        let Some(b) = byte.first() else {
            break;
        };
        raw.push(*b);
    }
    Ok(String::from_utf8_lossy(&raw).into_owned())
}

#[tokio::test]
async fn netctl_real_source_addr_is_seen_by_the_origin_server() {
    let origin = start_origin_v6().await.unwrap();
    let (ctl, proxy_port) = start_env(origin).await.unwrap();
    net_source_addr(&ctl, proxy_port, LOOPBACK_V6)
        .await
        .unwrap();
    let seen = get_via_proxy(proxy_port, origin, "/peer").await.unwrap();
    assert_eq!(seen.1, "::1");
    assert_eq!(
        net_mode(&ctl, proxy_port).await,
        NetMode::Open,
        "source tagging must keep the path open"
    );
}
