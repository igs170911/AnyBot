use serde_json::Value;

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

use anybot_app_test::serve::{Served, serve_start_with};
use anybot_domain::version_release::ReleaseTable;
use sqlx::Row;

mod pg_api_evidence_shared;
use pg_api_evidence_shared::{
    EVIDENCE_RECORD, FIRST_MESSAGE, act, bot_stream, conversation, create_bot, expect_accepted,
    fetch_record, login, record_count, send, stored,
};

const SECRET_TEXT: &str = "secret-evidence-line-91c4";

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

async fn shell_of(served: &Served, device: &str) -> Result<(i64, String, i64), String> {
    let row = sqlx::query("SELECT seq, kind, created_at_ms FROM records WHERE device_id = $1")
        .bind(device)
        .fetch_one(&served.pool)
        .await
        .map_err(|e| e.to_string())?;
    let seq: i64 = row.try_get("seq").map_err(|e| e.to_string())?;
    let kind: String = row.try_get("kind").map_err(|e| e.to_string())?;
    let at: i64 = row.try_get("created_at_ms").map_err(|e| e.to_string())?;
    Ok((seq, kind, at))
}

struct Seeded {
    message_seq: i64,
    evidence_seq: i64,
}

async fn seed(served: &Served, token: &str) -> Result<Seeded, String> {
    create_bot(served, token).await?;
    let items = conversation(SECRET_TEXT);
    let landed = send(served, token, "k-source-1", &bot_stream(), &items).await?;
    Ok(Seeded {
        message_seq: expect_accepted(&landed, 0)?,
        evidence_seq: expect_accepted(&landed, 3)?,
    })
}

async fn clear_and_purge(served: &Served, token: &str) -> Result<(), String> {
    let cleared = act(served, token, "k-source-2", "clear", None).await?;
    assert_eq!(cleared.status, 200, "{}", cleared.json);
    let clear_seq = cleared
        .json
        .get("seq")
        .and_then(Value::as_i64)
        .ok_or("no clear seq")?;
    let purged = act(
        served,
        token,
        "k-source-3",
        "purge_permanent",
        Some(clear_seq),
    )
    .await?;
    assert_eq!(purged.status, 200, "{}", purged.json);
    Ok(())
}

async fn assert_no_trace(served: &Served) -> Result<(), String> {
    let dump = dump_database(served).await?;
    assert!(
        !dump.contains(SECRET_TEXT),
        "plain text survived in the database"
    );
    let encoded = format!("{{\"text\":\"{SECRET_TEXT}\"}}");
    for candidate in [SECRET_TEXT.to_owned(), encoded] {
        for digest in digests_of(served, &candidate).await? {
            assert!(
                !dump.contains(&digest),
                "a digest of the content survived: {digest}"
            );
        }
    }
    Ok(())
}

async fn assert_evidence_source_is_gone(
    served: &Served,
    token: &str,
    seeded: &Seeded,
    before: &Value,
) -> Result<(), String> {
    let after = fetch_record(served, token, seeded.evidence_seq).await?;
    assert_eq!(after.status, 200, "{}", after.json);
    assert_eq!(&after.json, before, "the evidence is untouched");
    let cited = after
        .json
        .get("content")
        .and_then(|content| content.get("source_seq"))
        .and_then(Value::as_i64)
        .ok_or("the evidence names no source")?;
    let source = fetch_record(served, token, cited).await?;
    assert_eq!(source.status, 410, "{}", source.json);
    assert_eq!(source.code().as_deref(), Some("record-source-purged"));
    let (content, marked) = stored(served, EVIDENCE_RECORD).await?;
    assert!(!marked, "the evidence is not purged");
    assert!(content.contains("source_seq"));
    Ok(())
}

#[tokio::test]
async fn t_sync_append_4() -> Result<(), String> {
    let served = serve_start_with("evidence_source", &ReleaseTable::empty()).await?;
    let token = login(&served).await?;
    let seeded = seed(&served, &token).await?;
    let shell_before = shell_of(&served, FIRST_MESSAGE).await?;
    let evidence_before = fetch_record(&served, &token, seeded.evidence_seq).await?;
    assert_eq!(evidence_before.status, 200, "{}", evidence_before.json);
    clear_and_purge(&served, &token).await?;
    let shell_after = shell_of(&served, FIRST_MESSAGE).await?;
    assert_eq!(
        shell_after, shell_before,
        "device id, number, kind and time stay"
    );
    assert_eq!(shell_after.0, seeded.message_seq);
    let (content, marked) = stored(&served, FIRST_MESSAGE).await?;
    assert!(marked, "the message is marked permanently deleted");
    assert_eq!(content, "null", "the message content is empty");
    assert_evidence_source_is_gone(&served, &token, &seeded, &evidence_before.json).await?;
    assert_no_trace(&served).await?;
    assert!(record_count(&served).await? >= 6, "no record was removed");
    served.stop().await
}
