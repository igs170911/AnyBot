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
use anybot_app_test::sse::{data_json, open_sse};
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
async fn sse_carries_only_the_callers_own_streams() -> Result<(), String> {
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
    let mut stream = open_sse(&base, "/v1/streams/notifications", &bob, "").await?;
    let json = data_json(&stream.recv_block().await?)?;
    let keys: Vec<String> = json
        .get("streams")
        .and_then(Value::as_array)
        .map(|rows| {
            rows.iter()
                .filter_map(|r| r.get("stream").and_then(Value::as_str))
                .map(str::to_owned)
                .collect()
        })
        .unwrap_or_default();
    assert_eq!(
        keys,
        vec!["account/bob".to_owned()],
        "bob streams: {keys:?}"
    );
    stream.close();
    served.stop().await
}

#[tokio::test]
async fn sse_progress_lists_every_visible_stream() -> Result<(), String> {
    let served = serve_start_with("sse_visible", &ReleaseTable::empty()).await?;
    let base = served.base_url();
    let token = login_bearer(&served).await?;
    let create = vec![item(
        "dev-a",
        "bots_create",
        &json!({"bot_id": "bot-sse2", "name": "helper"}),
    )];
    let _landed = append(&base, &token, "k-sse-vis1", &append_body(&create)).await?;
    let bot_items = vec![item("dev-b", "message", &json!({"text": "bot note"}))];
    let bot_body = json!({"stream": {"bot": {"id": "bot-sse2"}}, "items": bot_items}).to_string();
    let _opened = append(&base, &token, "k-sse-vis2", &bot_body).await?;
    let mut stream = open_sse(&base, "/v1/streams/notifications", &token, "").await?;
    let json = data_json(&stream.recv_block().await?)?;
    let account_seq = stream_row(&json, "account/alice")?
        .get("seq")
        .and_then(Value::as_i64)
        .ok_or(format!("no account seq in {json}"))?;
    assert_eq!(account_seq, 1, "account max: {json}");
    assert_eq!(
        stream_row(&json, "bot/bot-sse2")?
            .get("seq")
            .and_then(Value::as_i64),
        Some(1),
        "bot max: {json}"
    );
    stream.close();
    served.stop().await
}
