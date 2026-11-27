use std::sync::Arc;
use std::sync::atomic::Ordering;
use std::time::Duration;

use serde_json::{Value, json};

use anybot_app_server as _;
use anybot_app_test as _;
use anybot_foundation as _;
use anybot_messages as _;
use anybot_server as _;
use anybot_server_test as _;
use anybot_store_server as _;
use axum as _;
use clap as _;
use rpassword as _;
use serde_json as _;
use sqlx as _;
use test_harness as _;
use tokio as _;
use tracing as _;

use anybot_app_test::clockwalk::ClockWalker;
use anybot_app_test::http::ResponseSpec;
use anybot_app_test::http::{RequestSpec, post};
use anybot_app_test::serve::serve_start_with;
use anybot_app_test::sse::{SseRequest, event_name, open_sse_via};
use anybot_app_test::sse_watch::{Target, watch};
use anybot_domain::version_release::ReleaseTable;
use test_harness::NetCtl;
use test_harness::netctl::{
    net_attach, net_buffer, net_idle_cut_clock, net_restore, net_share_clock, net_stall, net_start,
};

fn item(device: &str, kind: &str, content: &Value) -> Value {
    json!({
        "device_id": device,
        "kind": kind,
        "content": content,
        "created_at_ms": 1_700_000_000_100_i64,
        "offline_sent": false,
    })
}

fn append_body(items: &[Value]) -> String {
    json!({"stream": "account", "items": items}).to_string()
}

async fn login_pair(served: &anybot_app_test::serve::Served) -> Result<(String, String), String> {
    let base = served.base_url();
    anybot_app_test::serve::boot_pair(served).await?;
    let body = json!({
        "account": "alice",
        "password": "alicepassword",
        "label": "test-device",
        "credential": "bearer",
    })
    .to_string();
    let spec = RequestSpec::new(&body).with_key("k-hb-login");
    let mut walker = ClockWalker::start(&served.clock);
    let answer = post(&base, "/v1/account/login", &spec).await;
    walker.stop().await?;
    let login = answer?;
    let token = login.bearer()?;
    Ok((token, login.cookie.clone().unwrap_or_default()))
}

async fn append(base: &str, token: &str, key: &str, body: &str) -> Result<ResponseSpec, String> {
    let spec = RequestSpec::new(body).with_key(key).with_bearer(token);
    post(base, "/v1/streams/append", &spec).await
}

async fn open_proxy(served: &anybot_app_test::serve::Served) -> Result<(Arc<NetCtl>, u16), String> {
    let (ctl, port) = net_start().await?;
    net_attach(&ctl, port, served.origin_addr()).await?;
    Ok((ctl, port))
}

fn first_stall(
    base: &str,
    token: &str,
    cookie: &str,
    origin: std::net::SocketAddr,
    port: u16,
) -> Target {
    let mut made = Target::direct(base, "/v1/streams/notifications", token, "");
    made.cookie = Some(cookie.to_owned());
    made.stop_after_first = true;
    made.use_first.store(true, Ordering::SeqCst);
    made.first_origin = Some(origin);
    made.first_port = Some(port);
    made
}

fn restore_handler(
    ctl: Arc<NetCtl>,
    port: u16,
    target: Arc<Target>,
) -> anybot_app_test::sse_watch::OnReconnect {
    Box::new(move || {
        let owned = Arc::clone(&ctl);
        let switched = Arc::clone(&target);
        Box::pin(async move {
            let _opened = net_restore(&owned, port).await;
            switched.use_direct();
            test_harness::sleep_real(Duration::from_millis(300)).await;
        })
    })
}

#[tokio::test]
async fn t_conn_heartbeat_2() -> Result<(), String> {
    let served = serve_start_with("hb_2", &ReleaseTable::empty()).await?;
    let base = served.base_url();
    let (token, cookie) = login_pair(&served).await?;
    let (ctl, port) = open_proxy(&served).await?;
    net_stall(&ctl, port).await?;
    let target = Arc::new(first_stall(
        &base,
        &token,
        &cookie,
        served.origin_addr(),
        port,
    ));
    let shared = Arc::clone(&target);
    let mut pinned = (*target).clone();
    pinned.on_reconnect = Some(Arc::new(restore_handler(Arc::clone(&ctl), port, shared)));
    let owned = Arc::new(pinned);
    let mut walker = ClockWalker::start(&served.clock);
    let watched = watch(&owned, Duration::from_secs(48)).await?;
    let reconnects = watched.reconnects;
    let arrived = watched.arrived;
    watched.close();
    walker.stop().await?;
    ctl.cut_all().await?;
    ctl.drain_kills().await;
    served.stop().await?;
    assert_eq!(reconnects, 1, "reconnects: {reconnects}");
    assert!(arrived >= 1, "arrived blocks: {arrived}");
    Ok(())
}

#[tokio::test]
async fn t_conn_heartbeat_3() -> Result<(), String> {
    let served = serve_start_with("hb_3", &ReleaseTable::empty()).await?;
    let base = served.base_url();
    let (token, cookie) = login_pair(&served).await?;
    seed_one(&base, &token).await?;
    let (ctl, port) = open_proxy(&served).await?;
    net_share_clock(&ctl, port, &served.clock).await?;
    net_idle_cut_clock(&ctl, port, Duration::from_secs(100), &served.clock).await?;
    let mut stream = open_sse_via(
        served.origin_addr(),
        &SseRequest::new(&base, "/v1/streams/notifications", &token, "").with_cookie(&cookie),
        port,
    )
    .await?;
    let _head = stream.recv_block().await?;
    let mut walker = ClockWalker::manual(&served.clock);
    for slot in 0..40usize {
        let _beat = walker.step(Duration::from_secs(15)).await?;
        let block = stream.recv_block().await?;
        assert!(block.starts_with(':'), "slot {slot}: {block}");
    }
    let landed = append(
        &base,
        &token,
        "k-hb-3-live",
        &append_body(&[item("dev-hb3", "message", &json!({"text": "live"}))]),
    )
    .await?;
    assert_eq!(landed.status, 200, "live: {}", landed.json);
    let after = stream.recv_json_of("new-records").await?;
    assert!(after.get("seq").is_some(), "after: {after}");
    stream.close();
    ctl.cut_all().await?;
    ctl.drain_kills().await;
    served.stop().await
}

#[tokio::test]
async fn t_conn_heartbeat_4() -> Result<(), String> {
    let served = serve_start_with("hb_4", &ReleaseTable::empty()).await?;
    let base = served.base_url();
    let (token, cookie) = login_pair(&served).await?;
    let (ctl, port) = open_proxy(&served).await?;
    net_buffer(&ctl, port, true).await?;
    let mut stream = open_sse_via(
        served.origin_addr(),
        &SseRequest::new(&base, "/v1/streams/notifications", &token, "").with_cookie(&cookie),
        port,
    )
    .await?;
    let _head = stream.recv_block().await?;
    let started = test_harness::realtime::instant_now();
    let landed = append(
        &base,
        &token,
        "k-hb-4-push",
        &append_body(&[item("dev-hb4", "message", &json!({"text": "one"}))]),
    )
    .await?;
    assert_eq!(landed.status, 200, "push: {}", landed.json);
    let block = stream.recv_block().await?;
    let took = started.elapsed();
    assert_eq!(
        event_name(&block).as_deref(),
        Some("new-records"),
        "push: {block}"
    );
    let ms = u64::try_from(took.as_millis()).map_err(|_| "elapsed too large".to_owned())?;
    assert!(ms < 1000, "delivery took {ms} ms");
    stream.close();
    ctl.cut_all().await?;
    ctl.drain_kills().await;
    served.stop().await
}

async fn seed_one(base: &str, token: &str) -> Result<(), String> {
    let items = vec![item("dev-hb3-seed", "message", &json!({"text": "seed"}))];
    let answer = append(base, token, "k-hb-3-seed", &append_body(&items)).await?;
    assert_eq!(answer.status, 200, "seed: {}", answer.json);
    Ok(())
}
