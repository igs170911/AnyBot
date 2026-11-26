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
        Some(at) => format!("stream=account%2Falice&init_seq={at}"),
        None => "stream=account%2Falice".to_owned(),
    };
    open_sse(base, "/v1/streams/notifications", token, &query).await
}

fn progress_seq(json: &Value) -> Result<i64, String> {
    json.get("streams")
        .and_then(Value::as_array)
        .and_then(|rows| rows.first())
        .and_then(|row| row.get("seq"))
        .and_then(Value::as_i64)
        .ok_or(format!("no stream progress in {json}"))
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
    served.stop().await
}

#[tokio::test]
async fn sse_of_a_foreign_stream_is_refused() -> Result<(), String> {
    let served = serve_start_with("sse_foreign", &ReleaseTable::empty()).await?;
    let base = served.base_url();
    let token = login_bearer(&served).await?;
    let create = vec![item(
        "dev-a",
        "bots_create",
        &json!({"bot_id": "bot-sse", "name": "helper"}),
    )];
    let _landed = append(&base, &token, "k-sse-bot", &append_body(&create)).await?;
    let bob_body = json!({
        "account": "bob",
        "password": "bobpassword",
        "label": "test-device",
        "credential": "bearer",
    })
    .to_string();
    let spec = RequestSpec::new(&bob_body).with_key("k-sse-bob");
    let mut walker = ClockWalker::start(&served.clock);
    let landed = post(&base, "/v1/account/login", &spec).await;
    walker.stop().await?;
    let bob = landed?.bearer()?;
    let opened = open_sse(
        &base,
        "/v1/streams/notifications",
        &bob,
        "stream=bot%2Fbot-sse",
    )
    .await;
    assert!(opened.is_err(), "foreign sse must refuse: {opened:?}");
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
    let seen = stream.count_heartbeats(2).await?;
    assert!(seen >= 2, "heartbeats: {seen}");
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
    drop(stream);
    served.stop().await
}
