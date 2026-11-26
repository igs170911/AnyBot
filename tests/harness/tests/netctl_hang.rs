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
use reqwest as _;
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

use std::time::{Duration, Instant};

use test_harness::http::client_via_proxy;
use test_harness::netctl::{net_attach, net_hang_connect, net_start};
use test_harness::origin::start_origin;
use test_harness::realtime::timeout_real;

fn show<T, E: std::fmt::Display>(value: Result<T, E>) -> Result<T, String> {
    value.map_err(|err| format!("{err}"))
}

#[tokio::test]
async fn netctl_hang_connect_blocks_in_connect_not_refused() -> Result<(), String> {
    let origin = timeout_real(start_origin(), Duration::from_secs(20)).await??;
    let (ctl, proxy_port) = timeout_real(net_start(), Duration::from_secs(20)).await??;
    show(net_attach(&ctl, proxy_port, origin).await)?;
    show(net_hang_connect(&ctl, proxy_port).await)?;
    let client = show(client_via_proxy(&format!("http://127.0.0.1:{proxy_port}")))?;
    let started = Instant::now();
    let outcome = timeout_real(
        client.get(format!("http://{origin}/ok")).send(),
        Duration::from_millis(400),
    )
    .await;
    assert!(outcome.is_err(), "connect must still be pending");
    assert!(started.elapsed() >= Duration::from_millis(300));
    Ok(())
}

#[tokio::test]
async fn cloud_name_map_rewrites_only_the_aliased_host() -> Result<(), String> {
    let mut map = test_harness::NameMap::new();
    map.insert("api.cloud.test", show("127.0.0.1:9".parse())?);
    let mapped = map
        .resolve("http://api.cloud.test/v1/chat/completions")
        .ok_or("missing map")?;
    assert!(mapped.starts_with("http://127.0.0.1:9/"));
    assert!(map.resolve("http://api.other.test/v1").is_none());
    Ok(())
}
