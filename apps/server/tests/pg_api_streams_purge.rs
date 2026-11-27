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

use anybot_app_test::http::{RequestSpec, ResponseSpec, get, post};
use anybot_app_test::serve::{Served, boot_pair, serve_start_with};
use anybot_domain::version_release::ReleaseTable;
use sqlx::Row;

const HI_RECORD: &str = "018f0000-0000-7000-8000-0000000000c1";
const SECRET_RECORD: &str = "018f0000-0000-7000-8000-0000000000c2";
const SECRET_TEXT: &str = "secret-chat-line-7f3a";

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

async fn record_content(served: &Served, record: &str) -> Result<(String, bool), String> {
    let row = sqlx::query("SELECT content::text AS body, purged FROM records WHERE device_id = $1")
        .bind(record)
        .fetch_one(&served.pool)
        .await
        .map_err(|e| e.to_string())?;
    let body: String = row.try_get("body").map_err(|e| e.to_string())?;
    let purged: bool = row.try_get("purged").map_err(|e| e.to_string())?;
    Ok((body, purged))
}

async fn dump_database(served: &Served) -> Result<String, String> {
    let tables = sqlx::query(
        "SELECT table_name FROM information_schema.tables \
         WHERE table_schema = 'public' AND table_type = 'BASE TABLE'",
    )
    .fetch_all(&served.pool)
    .await
    .map_err(|e| e.to_string())?;
    let mut text = String::new();
    for table in tables {
        let name: String = table.try_get("table_name").map_err(|e| e.to_string())?;
        let sql = format!("SELECT row_to_json(t)::text AS line FROM \"{name}\" t");
        let rows = sqlx::query(sqlx::AssertSqlSafe(sql))
            .fetch_all(&served.pool)
            .await
            .map_err(|e| e.to_string())?;
        for row in rows {
            let line: String = row.try_get("line").map_err(|e| e.to_string())?;
            text.push_str(&line);
            text.push('\n');
        }
    }
    Ok(text)
}

async fn digests_of(served: &Served, text: &str) -> Result<Vec<String>, String> {
    let row = sqlx::query(
        "SELECT encode(sha256(convert_to($1, 'UTF8')), 'hex') AS hex_form, \
         encode(sha256(convert_to($1, 'UTF8')), 'base64') AS base64_form",
    )
    .bind(text)
    .fetch_one(&served.pool)
    .await
    .map_err(|e| e.to_string())?;
    let hex_form: String = row.try_get("hex_form").map_err(|e| e.to_string())?;
    let base64_form: String = row.try_get("base64_form").map_err(|e| e.to_string())?;
    Ok(vec![hex_form, base64_form])
}

#[tokio::test]
async fn t_sync_id_4() -> Result<(), String> {
    let served = serve_start_with("purge_id", &ReleaseTable::empty()).await?;
    let token = login_bearer(&served).await?;
    let first = send(&served, &token, "k-id-1", &[message(HI_RECORD, "hi")]).await?;
    assert_eq!(tag_at(&first, 0)?, "accepted", "{}", first.json);
    let original = seq_at(&first, 0)?;
    let changed = send(&served, &token, "k-id-2", &[message(HI_RECORD, "bye")]).await?;
    assert_eq!(tag_at(&changed, 0)?, "rejected", "{}", changed.json);
    assert_eq!(
        code_at(&changed, 0)?,
        "record-id-conflict",
        "{}",
        changed.json
    );
    let before_purge = record_count(&served).await?;
    let clear = act(&served, &token, "k-id-3", "clear", None).await?;
    assert_eq!(clear.status, 200, "{}", clear.json);
    let cleared = reply_seq(&clear)?;
    let purge = act(&served, &token, "k-id-4", "purge_permanent", Some(cleared)).await?;
    assert_eq!(purge.status, 200, "{}", purge.json);
    let after_purge = record_count(&served).await?;
    assert_eq!(
        after_purge,
        before_purge + 2,
        "clear and purge each add one record"
    );
    let again = send(&served, &token, "k-id-5", &[message(HI_RECORD, "hi")]).await?;
    assert_eq!(tag_at(&again, 0)?, "duplicate", "{}", again.json);
    assert_eq!(
        seq_at(&again, 0)?,
        original,
        "the original number comes back"
    );
    assert_eq!(
        record_count(&served).await?,
        after_purge,
        "nothing new was written"
    );
    let (body, purged) = record_content(&served, HI_RECORD).await?;
    assert!(purged, "the record stays marked purged");
    assert_eq!(body, "null", "the record is an empty shell");
    served.stop().await
}

#[tokio::test]
async fn purged_message_leaves_no_trace_in_the_database() -> Result<(), String> {
    let served = serve_start_with("purge_trace", &ReleaseTable::empty()).await?;
    let token = login_bearer(&served).await?;
    let secret = message(SECRET_RECORD, SECRET_TEXT);
    let first = send(&served, &token, "k-trace-1", &[secret]).await?;
    assert_eq!(tag_at(&first, 0)?, "accepted", "{}", first.json);
    let message_seq = seq_at(&first, 0)?;
    let clear = act(&served, &token, "k-trace-2", "clear", None).await?;
    let cleared = reply_seq(&clear)?;
    let purge = act(
        &served,
        &token,
        "k-trace-3",
        "purge_permanent",
        Some(cleared),
    )
    .await?;
    assert_eq!(purge.status, 200, "{}", purge.json);
    let source = get(
        &served.base_url(),
        &format!("/v1/streams/record?stream=account/alice&seq={message_seq}"),
        &RequestSpec::new("").with_bearer(&token),
    )
    .await?;
    assert_eq!(source.status, 410, "{}", source.json);
    assert_eq!(source.code().as_deref(), Some("record-source-purged"));
    let (body, purged) = record_content(&served, SECRET_RECORD).await?;
    assert!(purged, "the message is purged");
    assert_eq!(body, "null", "the content is gone");
    let dump = dump_database(&served).await?;
    assert!(
        !dump.contains(SECRET_TEXT),
        "plain text survived in the database"
    );
    let encoded = format!("{{\"text\":\"{SECRET_TEXT}\"}}");
    for candidate in [SECRET_TEXT.to_owned(), encoded] {
        for digest in digests_of(&served, &candidate).await? {
            assert!(
                !dump.contains(&digest),
                "a digest of the content survived: {digest}"
            );
        }
    }
    served.stop().await
}
