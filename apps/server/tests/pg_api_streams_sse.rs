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
use anybot_app_test::sse::{SseOpen, data_json, event_name, open_sse};
use anybot_domain::version_release::ReleaseTable;

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

async fn login_bearer(served: &anybot_app_test::serve::Served) -> Result<String, String> {
    let base = served.base_url();
    anybot_app_test::serve::boot_pair(served).await?;
    let body = json!({
        "account": "alice",
        "password": "alicepassword",
        "label": "test-device",
        "credential": "bearer",
    })
    .to_string();
    let spec = RequestSpec::new(&body).with_key("k-sse-login");
    let mut walker = ClockWalker::start(&served.clock);
    let answer = post(&base, "/v1/account/login", &spec).await;
    walker.stop().await?;
    answer?.bearer()
}

async fn append(base: &str, token: &str, key: &str, body: &str) -> Result<ResponseSpec, String> {
    let spec = RequestSpec::new(body).with_key(key).with_bearer(token);
    post(base, "/v1/streams/append", &spec).await
}

async fn seed(base: &str, token: &str, count: usize) -> Result<(), String> {
    for index in 0..count {
        let items = vec![item(
            &format!("dev-{index}"),
            "message",
            &json!({"text": index}),
        )];
        let answer = append(
            base,
            token,
            &format!("k-sse-seed-{index}"),
            &append_body(&items),
        )
        .await?;
        assert_eq!(answer.status, 200, "seed {index}: {}", answer.json);
    }
    Ok(())
}

async fn open_account(base: &str, token: &str, init: Option<i64>) -> Result<SseOpen, String> {
    let query = match init {
        Some(at) => format!("init_seq={at}"),
        None => String::new(),
    };
    open_sse(base, "/v1/streams/notifications", token, &query).await
}

fn progress_seq(json: &Value) -> Result<i64, String> {
    stream_row(json, "account/alice")?
        .get("seq")
        .and_then(Value::as_i64)
        .ok_or(format!("no account seq in {json}"))
}

fn stream_row<'a>(json: &'a Value, key: &str) -> Result<&'a Value, String> {
    let rows = json
        .get("streams")
        .and_then(Value::as_array)
        .ok_or(format!("no streams in {json}"))?;
    rows.iter()
        .find(|row| row.get("stream").and_then(Value::as_str) == Some(key))
        .ok_or(format!("no row for {key} in {json}"))
}

#[tokio::test]
async fn sse_first_event_is_progress_with_the_max() -> Result<(), String> {
    let served = serve_start_with("sse_progress", &ReleaseTable::empty()).await?;
    let base = served.base_url();
    let token = login_bearer(&served).await?;
    seed(&base, &token, 5).await?;
    let mut stream = open_account(&base, &token, None).await?;
    let block = stream.recv_block().await?;
    assert_eq!(
        event_name(&block).as_deref(),
        Some("progress"),
        "head: {block}"
    );
    let json = data_json(&block)?;
    assert_eq!(progress_seq(&json)?, 5, "head max: {json}");
    stream.close();
    served.stop().await
}

#[tokio::test]
async fn sse_pushes_new_records_after_an_append() -> Result<(), String> {
    let served = serve_start_with("sse_new_records", &ReleaseTable::empty()).await?;
    let base = served.base_url();
    let token = login_bearer(&served).await?;
    seed(&base, &token, 2).await?;
    let mut stream = open_account(&base, &token, None).await?;
    let _head = stream.recv_block().await?;
    let items = vec![item(
        "dev-new",
        "message",
        &json!({"text": "blue-dolphin-42"}),
    )];
    let landed = append(&base, &token, "k-sse-push", &append_body(&items)).await?;
    assert_eq!(landed.status, 200);
    let json = stream.recv_json_of("new-records").await?;
    let seq = json
        .get("seq")
        .and_then(Value::as_i64)
        .ok_or(format!("no seq in {json}"))?;
    assert_eq!(seq, 3, "pushed seq: {json}");
    assert_eq!(
        json.get("device_id").and_then(Value::as_str),
        Some("dev-new")
    );
    stream.close();
    served.stop().await
}

#[tokio::test]
async fn sse_init_seq_replays_the_missed_records() -> Result<(), String> {
    let served = serve_start_with("sse_catch_up", &ReleaseTable::empty()).await?;
    let base = served.base_url();
    let token = login_bearer(&served).await?;
    seed(&base, &token, 4).await?;
    let mut stream = open_account(&base, &token, Some(2)).await?;
    let _head = stream.recv_block().await?;
    let first = stream.recv_block().await?;
    assert_eq!(
        event_name(&first).as_deref(),
        Some("new-records"),
        "catch: {first}"
    );
    let json = data_json(&first)?;
    let seq = json
        .get("seq")
        .and_then(Value::as_i64)
        .ok_or(format!("no seq in {json}"))?;
    assert_eq!(seq, 3, "replay starts at 3: {json}");
    stream.close();
    served.stop().await
}

#[tokio::test]
async fn sse_heartbeats_arrive_every_fifteen_seconds() -> Result<(), String> {
    let served = serve_start_with("sse_heartbeat", &ReleaseTable::empty()).await?;
    let base = served.base_url();
    let token = login_bearer(&served).await?;
    seed(&base, &token, 1).await?;
    let mut stream = open_account(&base, &token, None).await?;
    let _head = stream.recv_block().await?;
    let mut walker = ClockWalker::start(&served.clock);
    let seen = stream.count_heartbeats(2).await?;
    walker.stop().await?;
    assert!(seen >= 2, "heartbeats: {seen}");
    stream.close();
    served.stop().await
}

async fn post_ok(
    base: &str,
    token: &str,
    key: &str,
    path: &str,
    body: &str,
) -> Result<ResponseSpec, String> {
    let spec = RequestSpec::new(body).with_key(key).with_bearer(token);
    post(base, path, &spec).await
}

#[tokio::test]
async fn sse_closes_within_five_seconds_when_credentials_are_revoked() -> Result<(), String> {
    let served = serve_start_with("sse_revoke", &ReleaseTable::empty()).await?;
    let base = served.base_url();
    let token = login_bearer(&served).await?;
    seed(&base, &token, 1).await?;
    let mut stream = open_account(&base, &token, None).await?;
    let _head = stream.recv_block().await?;
    let logout = post_ok(
        &base,
        &token,
        "k-sse-logout",
        "/v1/account/logout",
        r#"{"ok": true}"#,
    )
    .await?;
    assert_eq!(logout.status, 200, "logout: {}", logout.json);
    let mut walker = ClockWalker::start(&served.clock);
    let ended = stream.recv_ended(std::time::Duration::from_secs(5)).await;
    walker.stop().await?;
    ended.map_err(|e| format!("revoked sse must end: {e}"))?;
    stream.close();
    served.stop().await
}

#[tokio::test]
async fn sse_carries_the_accel_buffering_header() -> Result<(), String> {
    let served = serve_start_with("sse_accel", &ReleaseTable::empty()).await?;
    let base = served.base_url();
    let token = login_bearer(&served).await?;
    seed(&base, &token, 1).await?;
    let stream = open_account(&base, &token, None).await?;
    let values = stream
        .headers
        .get_all("x-accel-buffering")
        .iter()
        .map(|v| v.to_str().unwrap_or_default().to_owned())
        .collect::<Vec<String>>();
    assert_eq!(values, vec!["no".to_owned()], "accel header: {values:?}");
    stream.close();
    served.stop().await
}
