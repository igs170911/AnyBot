use anybot_protocol::ToolEffect;
use serde_json::json;

use crate::{ACCOUNT_STREAM, DATABASE_FILE, JournalEntry, NewOutbox, RecoveryReason, Store};

fn pending(record_id: &str) -> NewOutbox {
    NewOutbox {
        stream: ACCOUNT_STREAM.to_owned(),
        record_id: record_id.to_owned(),
        kind: "message".to_owned(),
        content: json!({"n": record_id}),
        created_at_ms: 1,
        task_id: None,
        offline_at_create: false,
    }
}

#[tokio::test]
async fn file_that_is_not_sqlite_is_moved_aside_byte_identical() -> Result<(), String> {
    let dir = test_harness::test_dir("store_client_not_sqlite")?;
    let garbage = b"this is definitely not a database file, just text padding".repeat(40);
    std::fs::write(dir.join(DATABASE_FILE), &garbage).map_err(|e| e.to_string())?;
    let opened = Store::open(&dir, 77).await.map_err(|e| e.to_string())?;
    let recovery = opened.recovery.ok_or("no recovery reported")?;
    assert!(matches!(recovery.reason, RecoveryReason::NotSqlite(_)));
    assert_eq!(recovery.reason.code(), "replica-not-sqlite");
    let moved = std::fs::read(&recovery.moved_to).map_err(|e| e.to_string())?;
    assert_eq!(moved, garbage);
    assert!(
        opened
            .store
            .outbox_all()
            .await
            .map_err(|e| e.to_string())?
            .is_empty()
    );
    Ok(())
}

#[tokio::test]
async fn newer_schema_is_moved_aside_and_outbox_is_rescued() -> Result<(), String> {
    let dir = test_harness::test_dir("store_client_newer_schema")?;
    let first = Store::open(&dir, 1).await.map_err(|e| e.to_string())?;
    first
        .store
        .enqueue(&pending("rescue-me"))
        .await
        .map_err(|e| e.to_string())?;
    sqlx::query(
        "INSERT INTO _sqlx_migrations (version, description, success, checksum, execution_time)
            VALUES (9999, 'future', 1, x'00', 0)",
    )
    .execute(first.store.pool())
    .await
    .map_err(|e| e.to_string())?;
    first.store.close().await;
    let reopened = Store::open(&dir, 88).await.map_err(|e| e.to_string())?;
    let recovery = reopened.recovery.ok_or("no recovery reported")?;
    assert_eq!(recovery.reason.code(), "replica-newer-schema");
    assert_eq!(recovery.rescued_outbox, 1);
    assert!(!recovery.outbox_unreadable);
    let rows = reopened
        .store
        .outbox_all()
        .await
        .map_err(|e| e.to_string())?;
    assert_eq!(
        rows.first().map(|row| row.record_id.as_str()),
        Some("rescue-me")
    );
    Ok(())
}

#[tokio::test]
async fn damaged_pages_are_moved_aside_and_a_new_database_starts() -> Result<(), String> {
    let dir = test_harness::test_dir("store_client_damaged")?;
    let first = Store::open(&dir, 1).await.map_err(|e| e.to_string())?;
    first
        .store
        .enqueue(&pending("survivor"))
        .await
        .map_err(|e| e.to_string())?;
    first.store.close().await;
    let path = dir.join(DATABASE_FILE);
    let mut bytes = std::fs::read(&path).map_err(|e| e.to_string())?;
    let damaged_from = bytes.len().saturating_sub(4096).max(4096);
    for byte in bytes.iter_mut().skip(damaged_from) {
        *byte = 0xA5;
    }
    std::fs::write(&path, &bytes).map_err(|e| e.to_string())?;
    let reopened = Store::open(&dir, 99).await.map_err(|e| e.to_string())?;
    let recovery = reopened.recovery.ok_or("no recovery reported")?;
    assert_eq!(recovery.reason.code(), "replica-damaged");
    let moved = std::fs::read(&recovery.moved_to).map_err(|e| e.to_string())?;
    assert_eq!(moved, bytes);
    Ok(())
}

#[tokio::test]
async fn recovered_journal_row_keeps_turn_and_local_detail() -> Result<(), String> {
    let dir = test_harness::test_dir("store_recover_journal_cols")?;
    let opened = Store::open(&dir, 1).await.map_err(|e| e.to_string())?;
    opened
        .store
        .journal_append(&open_turn())
        .await
        .map_err(|e| e.to_string())?;
    mark_future_migration(&opened.store).await?;
    opened.store.close().await;
    let reopened = Store::open(&dir, 88).await.map_err(|e| e.to_string())?;
    let recovery = reopened.recovery.ok_or("no recovery reported")?;
    assert_eq!(recovery.reason.code(), "replica-newer-schema");
    let open = reopened
        .store
        .journal_open()
        .await
        .map_err(|e| e.to_string())?;
    let row = open.first().ok_or("open turn missing after recovery")?;
    assert_eq!(row.kind, "turn_started");
    assert_eq!(row.turn, Some(4));
    assert_eq!(row.local_detail.as_deref(), Some("keep-raw"));
    assert_eq!(row.effect, Some(ToolEffect::ReadOnly));
    Ok(())
}

fn open_turn() -> JournalEntry {
    JournalEntry {
        stream: crate::bot_stream("bot-recover"),
        record_id: "rec-turn".to_owned(),
        kind: "turn_started".to_owned(),
        content: json!({"kind": "turn_started"}),
        local_detail: Some("keep-raw".to_owned()),
        created_at_ms: 1_700_000_000_002,
        task_id: Some("task-1".to_owned()),
        turn: Some(4),
        effect: Some(ToolEffect::ReadOnly),
        offline_at_create: false,
    }
}

async fn mark_future_migration(store: &Store) -> Result<(), String> {
    sqlx::query(
        "INSERT INTO _sqlx_migrations (version, description, success, checksum, execution_time)
            VALUES (9999, 'future', 1, x'00', 0)",
    )
    .execute(store.pool())
    .await
    .map(|_| ())
    .map_err(|e| e.to_string())
}
