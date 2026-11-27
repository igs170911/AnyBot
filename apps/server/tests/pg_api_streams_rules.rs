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

use std::time::Duration;

use anybot_app_test::http::{RequestSpec, ResponseSpec, post};
use anybot_app_test::serve::{Served, boot_pair, serve_start_with};
use anybot_domain::version_release::ReleaseTable;
use sqlx::Row;

const FIRST_RECORD: &str = "018f0000-0000-7000-8000-0000000000d1";
const SECOND_RECORD: &str = "018f0000-0000-7000-8000-0000000000d2";
const RESTORE_WINDOW_DAYS: u64 = 31;

fn plain_item(record: &str, kind: &str) -> Value {
    json!({
        "device_id": record,
        "kind": kind,
        "content": {},
        "created_at_ms": 1_700_000_000_100_i64,
        "offline_sent": false,
    })
}

async fn login_bearer(served: &Served) -> Result<String, String> {
    let base = served.base_url();
    boot_pair(served).await?;
    let body = json!({
        "account": "alice",
        "password": "alicepassword",
        "label": "test-device",
        "credential": "bearer",
    })
    .to_string();
    let spec = RequestSpec::new(&body).with_key("k-purge-login");
    post(&base, "/v1/account/login", &spec).await?.bearer()
}

fn message(record: &str, text: &str) -> Value {
    json!({
        "device_id": record,
        "kind": "message",
        "content": {"text": text},
        "created_at_ms": 1_700_000_000_100_i64,
        "offline_sent": false,
    })
}

async fn send(
    served: &Served,
    token: &str,
    key: &str,
    items: &[Value],
) -> Result<ResponseSpec, String> {
    let body = json!({"stream": "account", "items": items}).to_string();
    let spec = RequestSpec::new(&body).with_key(key).with_bearer(token);
    post(&served.base_url(), "/v1/streams/append", &spec).await
}

async fn act(
    served: &Served,
    token: &str,
    key: &str,
    action: &str,
    target: Option<i64>,
) -> Result<ResponseSpec, String> {
    let body = match target {
        Some(seq) => json!({"stream": "account", "action": action, "target_seq": seq}),
        None => json!({"stream": "account", "action": action}),
    };
    let spec = RequestSpec::new(&body.to_string())
        .with_key(key)
        .with_bearer(token);
    post(&served.base_url(), "/v1/streams/actions", &spec).await
}

fn result_row(answer: &ResponseSpec, index: usize) -> Result<&Value, String> {
    answer
        .json
        .get("results")
        .and_then(Value::as_array)
        .and_then(|rows| rows.get(index))
        .ok_or(format!("no result {index} in {}", answer.json))
}

fn tag_at(answer: &ResponseSpec, index: usize) -> Result<String, String> {
    result_row(answer, index)?
        .get("result")
        .and_then(Value::as_str)
        .map(str::to_owned)
        .ok_or(format!("no result tag at {index}: {}", answer.json))
}

fn code_at(answer: &ResponseSpec, index: usize) -> Result<String, String> {
    result_row(answer, index)?
        .get("code")
        .and_then(Value::as_str)
        .map(str::to_owned)
        .ok_or(format!("no code at {index}: {}", answer.json))
}

fn seq_at(answer: &ResponseSpec, index: usize) -> Result<i64, String> {
    result_row(answer, index)?
        .get("seq")
        .and_then(Value::as_i64)
        .ok_or(format!("no seq at {index}: {}", answer.json))
}

fn reply_seq(answer: &ResponseSpec) -> Result<i64, String> {
    answer
        .json
        .get("seq")
        .and_then(Value::as_i64)
        .ok_or(format!("no seq in {}", answer.json))
}

async fn record_count(served: &Served) -> Result<i64, String> {
    let row = sqlx::query("SELECT count(*) AS n FROM records")
        .fetch_one(&served.pool)
        .await
        .map_err(|e| e.to_string())?;
    row.try_get::<i64, _>("n").map_err(|e| e.to_string())
}

#[tokio::test]
async fn forged_chat_actions_and_early_purge_are_refused() -> Result<(), String> {
    let served = serve_start_with("purge_rules", &ReleaseTable::empty()).await?;
    let token = login_bearer(&served).await?;
    let two = send(
        &served,
        &token,
        "k-rules-1",
        &[message(FIRST_RECORD, "one"), message(SECOND_RECORD, "two")],
    )
    .await?;
    assert_eq!(tag_at(&two, 0)?, "accepted", "{}", two.json);
    let first_message = seq_at(&two, 0)?;
    let before_forged = record_count(&served).await?;
    let forged = send(
        &served,
        &token,
        "k-rules-2",
        &[
            plain_item("018f0000-0000-7000-8000-0000000000e1", "record_purge"),
            plain_item("018f0000-0000-7000-8000-0000000000e2", "record_restore"),
            plain_item(
                "018f0000-0000-7000-8000-0000000000e3",
                "record_purge_permanent",
            ),
        ],
    )
    .await?;
    for index in 0..3 {
        assert_eq!(tag_at(&forged, index)?, "rejected", "{}", forged.json);
        assert_eq!(
            code_at(&forged, index)?,
            "append-kind-forbidden",
            "{}",
            forged.json
        );
    }
    assert_eq!(
        record_count(&served).await?,
        before_forged,
        "forged actions wrote nothing"
    );
    let early = act(
        &served,
        &token,
        "k-rules-3",
        "purge_permanent",
        Some(first_message),
    )
    .await?;
    assert_eq!(early.status, 400, "{}", early.json);
    assert_eq!(
        early.code().as_deref(),
        Some("record-action-target-invalid")
    );
    served.stop().await
}
#[tokio::test]
async fn restore_window_closes_after_thirty_one_days() -> Result<(), String> {
    let served = serve_start_with("purge_window", &ReleaseTable::empty()).await?;
    let token = login_bearer(&served).await?;
    let seed = send(
        &served,
        &token,
        "k-window-0",
        &[message(FIRST_RECORD, "one")],
    )
    .await?;
    assert_eq!(tag_at(&seed, 0)?, "accepted", "{}", seed.json);
    let clear = act(&served, &token, "k-window-1", "clear", None).await?;
    assert_eq!(clear.status, 200, "{}", clear.json);
    let cleared = reply_seq(&clear)?;
    let _advanced = served
        .clock
        .advance(Duration::from_secs(RESTORE_WINDOW_DAYS * 24 * 60 * 60 + 1));
    let late = act(&served, &token, "k-window-2", "restore", Some(cleared)).await?;
    assert_eq!(late.status, 409, "{}", late.json);
    assert_eq!(late.code().as_deref(), Some("record-restore-expired"));
    served.stop().await
}
