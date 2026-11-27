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
use test_harness as _;
use tokio as _;
use tracing as _;

use anybot_app_test::clockwalk::ClockWalker;
use anybot_app_test::http::{RequestSpec, ResponseSpec, post};
use anybot_app_test::serve::{Served, boot_pair, serve_start_with};
use anybot_domain::version_release::ReleaseTable;
use sqlx::Row;

async fn login(
    served: &Served,
    account: &str,
    password: &str,
    key: &str,
) -> Result<String, String> {
    let body = json!({
        "account": account,
        "password": password,
        "label": "test-device",
        "credential": "bearer",
    })
    .to_string();
    let spec = RequestSpec::new(&body).with_key(key);
    let mut walker = ClockWalker::start(&served.clock);
    let answer = post(&served.base_url(), "/v1/account/login", &spec).await;
    walker.stop().await?;
    answer?.bearer()
}

fn account_item(record: &str, kind: &str, content: &Value) -> Value {
    json!({
        "device_id": record,
        "kind": kind,
        "content": content,
        "created_at_ms": 1_700_000_000_100_i64,
        "offline_sent": false,
    })
}

async fn send_account(
    served: &Served,
    token: &str,
    key: &str,
    items: &[Value],
) -> Result<ResponseSpec, String> {
    let body = json!({"stream": "account", "items": items}).to_string();
    let spec = RequestSpec::new(&body).with_key(key).with_bearer(token);
    post(&served.base_url(), "/v1/streams/append", &spec).await
}

fn tag_at(answer: &ResponseSpec, index: usize) -> Result<String, String> {
    answer
        .json
        .get("results")
        .and_then(Value::as_array)
        .and_then(|rows| rows.get(index))
        .and_then(|row| row.get("result"))
        .and_then(Value::as_str)
        .map(str::to_owned)
        .ok_or(format!("no result tag at {index}: {}", answer.json))
}

async fn count_of(served: &Served, sql: &'static str) -> Result<i64, String> {
    let row = sqlx::query(sql)
        .fetch_one(&served.pool)
        .await
        .map_err(|e| e.to_string())?;
    row.try_get::<i64, _>("n").map_err(|e| e.to_string())
}

async fn bot_deleted(served: &Served, bot: &str) -> Result<bool, String> {
    let row = sqlx::query("SELECT deleted FROM bots WHERE id = $1")
        .bind(bot)
        .fetch_one(&served.pool)
        .await
        .map_err(|e| e.to_string())?;
    row.try_get::<bool, _>("deleted").map_err(|e| e.to_string())
}

#[tokio::test]
async fn bots_create_writes_the_record_the_bot_row_and_the_bot_stream_together()
-> Result<(), String> {
    let served = serve_start_with("projection_create", &ReleaseTable::empty()).await?;
    boot_pair(&served).await?;
    let token = login(&served, "alice", "alicepassword", "k-proj-login").await?;
    let create = account_item(
        "dev-create",
        "bots_create",
        &json!({"bot_id": "bot-p1", "name": "helper"}),
    );
    let landed = send_account(&served, &token, "k-proj-1", &[create]).await?;
    assert_eq!(tag_at(&landed, 0)?, "accepted", "{}", landed.json);
    let records = count_of(&served, "SELECT count(*) AS n FROM records").await?;
    let bots = count_of(
        &served,
        "SELECT count(*) AS n FROM bots WHERE id = 'bot-p1'",
    )
    .await?;
    let streams = count_of(
        &served,
        "SELECT count(*) AS n FROM record_streams WHERE id = 'bot/bot-p1'",
    )
    .await?;
    assert_eq!((records, bots, streams), (1, 1, 1));
    served.stop().await
}

#[tokio::test]
async fn a_projection_that_fails_takes_the_record_and_its_number_back() -> Result<(), String> {
    let served = serve_start_with("projection_rollback", &ReleaseTable::empty()).await?;
    boot_pair(&served).await?;
    let token = login(&served, "alice", "alicepassword", "k-proj-login").await?;
    let nameless = account_item("dev-broken", "bots_create", &json!({"bot_id": "bot-p2"}));
    let broken = send_account(&served, &token, "k-proj-2", &[nameless]).await?;
    assert_eq!(broken.status, 500, "{}", broken.json);
    assert_eq!(
        count_of(&served, "SELECT count(*) AS n FROM records").await?,
        0,
        "the record of the failed projection must not stay"
    );
    let note = account_item("dev-note", "message", &json!({"text": "after"}));
    let next = send_account(&served, &token, "k-proj-3", &[note]).await?;
    let seq = next
        .json
        .get("results")
        .and_then(Value::as_array)
        .and_then(|rows| rows.first())
        .and_then(|row| row.get("seq"))
        .and_then(Value::as_i64);
    assert_eq!(seq, Some(1), "no number was burnt: {}", next.json);
    served.stop().await
}

#[tokio::test]
async fn bots_delete_from_another_account_leaves_the_bot_alone() -> Result<(), String> {
    let served = serve_start_with("projection_foreign", &ReleaseTable::empty()).await?;
    boot_pair(&served).await?;
    let alice = login(&served, "alice", "alicepassword", "k-proj-login-a").await?;
    let bob = login(&served, "bob", "bobpassword", "k-proj-login-b").await?;
    let create = account_item(
        "dev-create",
        "bots_create",
        &json!({"bot_id": "bot-p3", "name": "helper"}),
    );
    let landed = send_account(&served, &alice, "k-proj-4", &[create]).await?;
    assert_eq!(tag_at(&landed, 0)?, "accepted", "{}", landed.json);
    let foreign = account_item("dev-foreign", "bots_delete", &json!({"bot_id": "bot-p3"}));
    let _sent = send_account(&served, &bob, "k-proj-5", &[foreign]).await?;
    assert!(
        !bot_deleted(&served, "bot-p3").await?,
        "bob deleted alice's bot"
    );
    let own = account_item("dev-own", "bots_delete", &json!({"bot_id": "bot-p3"}));
    let _sent = send_account(&served, &alice, "k-proj-6", &[own]).await?;
    assert!(bot_deleted(&served, "bot-p3").await?);
    let restore = account_item("dev-back", "bots_restore", &json!({"bot_id": "bot-p3"}));
    let _sent = send_account(&served, &alice, "k-proj-7", &[restore]).await?;
    assert!(!bot_deleted(&served, "bot-p3").await?);
    served.stop().await
}

#[tokio::test]
async fn one_account_can_create_two_bots() -> Result<(), String> {
    let served = serve_start_with("projection_two_bots", &ReleaseTable::empty()).await?;
    boot_pair(&served).await?;
    let token = login(&served, "alice", "alicepassword", "k-proj-login").await?;
    let first = account_item(
        "dev-first",
        "bots_create",
        &json!({"bot_id": "bot-p4", "name": "one"}),
    );
    let second = account_item(
        "dev-second",
        "bots_create",
        &json!({"bot_id": "bot-p5", "name": "two"}),
    );
    let landed = send_account(&served, &token, "k-proj-8", &[first, second]).await?;
    assert_eq!(tag_at(&landed, 0)?, "accepted", "{}", landed.json);
    assert_eq!(tag_at(&landed, 1)?, "accepted", "{}", landed.json);
    let bots = count_of(&served, "SELECT count(*) AS n FROM bots").await?;
    assert_eq!(bots, 2);
    served.stop().await
}
