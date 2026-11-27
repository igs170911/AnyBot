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

use anybot_app_test::http::{RequestSpec, ResponseSpec, get, post};
use anybot_app_test::serve::{Served, boot_pair, serve_start_with};
use anybot_domain::version_release::ReleaseTable;
use sqlx::Row;

const PHONE: &str = "018f0000-0000-7000-8000-0000000000b1";
const LAPTOP_FIRST: &str = "018f0000-0000-7000-8000-0000000000a1";
const LAPTOP_SECOND: &str = "018f0000-0000-7000-8000-0000000000a2";
const MINUTE_MS: i64 = 60_000;
const MINUTE: Duration = Duration::from_secs(60);

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
    let spec = RequestSpec::new(&body).with_key("k-settings-login");
    post(&base, "/v1/account/login", &spec).await?.bearer()
}

fn setting(field: &str, value: &str, changed_at_ms: i64, offset_ms: i64) -> Value {
    json!({
        "field": field,
        "value": value,
        "changed_at_ms": changed_at_ms,
        "offset_ms": offset_ms,
    })
}

fn setting_item(record: &str, offline: bool, content: &Value) -> Value {
    json!({
        "device_id": record,
        "kind": "setting",
        "content": content,
        "created_at_ms": 1_700_000_000_100_i64,
        "offline_sent": offline,
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
    let answer = post(&served.base_url(), "/v1/streams/append", &spec).await?;
    assert_eq!(answer.status, 200, "append: {}", answer.json);
    Ok(answer)
}

fn tag_at(answer: &ResponseSpec, index: usize) -> Result<String, String> {
    let row = answer
        .json
        .get("results")
        .and_then(Value::as_array)
        .and_then(|rows| rows.get(index))
        .ok_or(format!("no result {index} in {}", answer.json))?;
    row.get("result")
        .and_then(Value::as_str)
        .map(str::to_owned)
        .ok_or(format!("no result tag in {row}"))
}

fn code_at(answer: &ResponseSpec, index: usize) -> Result<String, String> {
    let row = answer
        .json
        .get("results")
        .and_then(Value::as_array)
        .and_then(|rows| rows.get(index))
        .ok_or(format!("no result {index} in {}", answer.json))?;
    row.get("code")
        .and_then(Value::as_str)
        .map(str::to_owned)
        .ok_or(format!("no code in {row}"))
}

async fn winner(served: &Served, token: &str, field: &str) -> Result<String, String> {
    let spec = RequestSpec::new("").with_bearer(token);
    let view = get(&served.base_url(), "/v1/settings", &spec).await?;
    assert_eq!(view.status, 200, "settings: {}", view.json);
    let rows = view
        .json
        .get("settings")
        .and_then(Value::as_array)
        .ok_or(format!("no settings in {}", view.json))?;
    rows.iter()
        .find(|row| row.get("field").and_then(Value::as_str) == Some(field))
        .and_then(|row| row.get("value"))
        .and_then(Value::as_str)
        .map(str::to_owned)
        .ok_or(format!("no winner for {field} in {}", view.json))
}

async fn corrected_of(served: &Served, record: &str) -> Result<i64, String> {
    let row = sqlx::query("SELECT corrected_ms FROM records WHERE device_id = $1")
        .bind(record)
        .fetch_one(&served.pool)
        .await
        .map_err(|e| e.to_string())?;
    row.try_get::<Option<i64>, _>("corrected_ms")
        .map_err(|e| e.to_string())?
        .ok_or(format!("record {record} has no corrected time"))
}

async fn record_count(served: &Served) -> Result<i64, String> {
    let row = sqlx::query("SELECT count(*) AS n FROM records")
        .fetch_one(&served.pool)
        .await
        .map_err(|e| e.to_string())?;
    row.try_get::<i64, _>("n").map_err(|e| e.to_string())
}

#[tokio::test]
async fn t_sync_settings_2() -> Result<(), String> {
    let served = serve_start_with("settings_late", &ReleaseTable::empty()).await?;
    let token = login_bearer(&served).await?;
    let start = i64::try_from(served.clock.now_ms()).map_err(|e| e.to_string())?;
    let green = setting_item(
        LAPTOP_FIRST,
        true,
        &setting("accent_color", "green", start, 0),
    );
    let _advanced = served.clock.advance(5 * MINUTE);
    let red_now = start + 5 * MINUTE_MS;
    let red = setting_item(PHONE, false, &setting("accent_color", "red", red_now, 0));
    let phone_answer = send(&served, &token, "k-late-p", &[red]).await?;
    assert_eq!(
        tag_at(&phone_answer, 0)?,
        "accepted",
        "{}",
        phone_answer.json
    );
    let _advanced = served.clock.advance(5 * MINUTE);
    let laptop = send(&served, &token, "k-late-a", &[green]).await?;
    assert_eq!(tag_at(&laptop, 0)?, "accepted", "{}", laptop.json);
    let value = winner(&served, &token, "accent_color").await?;
    assert_eq!(value, "red", "later edit must win after the late upload");
    served.stop().await
}

#[tokio::test]
async fn t_sync_settings_3() -> Result<(), String> {
    let served = serve_start_with("settings_quiet", &ReleaseTable::empty()).await?;
    let token = login_bearer(&served).await?;
    let start = i64::try_from(served.clock.now_ms()).map_err(|e| e.to_string())?;
    let green = setting_item(
        LAPTOP_FIRST,
        true,
        &setting("accent_color", "green", start, 0),
    );
    let _advanced = served.clock.advance(10 * MINUTE);
    let answer = send(&served, &token, "k-quiet-a", &[green]).await?;
    assert_eq!(tag_at(&answer, 0)?, "accepted", "{}", answer.json);
    assert_eq!(winner(&served, &token, "accent_color").await?, "green");
    served.stop().await
}

#[tokio::test]
async fn t_sync_settings_6() -> Result<(), String> {
    let served = serve_start_with("settings_skew", &ReleaseTable::empty()).await?;
    let token = login_bearer(&served).await?;
    let start = i64::try_from(served.clock.now_ms()).map_err(|e| e.to_string())?;
    let fast_hour = start + 60 * MINUTE_MS;
    let green = setting_item(
        LAPTOP_FIRST,
        false,
        &setting("accent_color", "green", fast_hour, 0),
    );
    let first = send(&served, &token, "k-skew-a1", &[green]).await?;
    assert_eq!(tag_at(&first, 0)?, "accepted", "{}", first.json);
    let _advanced = served.clock.advance(MINUTE);
    let red = setting_item(
        PHONE,
        false,
        &setting("accent_color", "red", start + MINUTE_MS, 0),
    );
    let phone = send(&served, &token, "k-skew-p", &[red]).await?;
    assert_eq!(tag_at(&phone, 0)?, "accepted", "{}", phone.json);
    let loose = setting_item(
        LAPTOP_SECOND,
        false,
        &setting("density", "loose", fast_hour, 0),
    );
    let compact = setting_item(
        "018f0000-0000-7000-8000-0000000000a3",
        false,
        &setting("density", "compact", fast_hour, 0),
    );
    let pair = send(&served, &token, "k-skew-a2", &[loose, compact]).await?;
    assert_eq!(tag_at(&pair, 0)?, "accepted", "{}", pair.json);
    assert_eq!(tag_at(&pair, 1)?, "accepted", "{}", pair.json);
    assert_eq!(winner(&served, &token, "accent_color").await?, "red");
    assert_eq!(winner(&served, &token, "density").await?, "compact");
    let received = start;
    assert_eq!(
        corrected_of(&served, LAPTOP_FIRST).await?,
        received,
        "a fast clock is capped at the server receive time"
    );
    served.stop().await
}

#[tokio::test]
async fn t_sync_settings_7() -> Result<(), String> {
    let served = serve_start_with("settings_refused", &ReleaseTable::empty()).await?;
    let token = login_bearer(&served).await?;
    let before = record_count(&served).await?;
    let start = i64::try_from(served.clock.now_ms()).map_err(|e| e.to_string())?;
    let device_only = setting_item(LAPTOP_FIRST, false, &setting("theme", "dark", start, 0));
    let unknown = setting_item(
        LAPTOP_SECOND,
        false,
        &setting("accentcolour", "green", start, 0),
    );
    let answer = send(&served, &token, "k-refuse", &[device_only, unknown]).await?;
    assert_eq!(tag_at(&answer, 0)?, "rejected", "{}", answer.json);
    assert_eq!(
        code_at(&answer, 0)?,
        "setting-device-only",
        "{}",
        answer.json
    );
    assert_eq!(tag_at(&answer, 1)?, "rejected", "{}", answer.json);
    assert_eq!(code_at(&answer, 1)?, "setting-unknown", "{}", answer.json);
    assert_eq!(
        record_count(&served).await?,
        before,
        "no record was written"
    );
    served.stop().await
}
