use serde_json::json;

use crate::{ACCOUNT_STREAM, DATABASE_FILE, NewOutbox, RecoveryReason, Store};

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
