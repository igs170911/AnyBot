use anybot_protocol::ToolEffect;
use serde_json::json;

use crate::{JournalAppend, JournalEntry, OUTBOX_SEQ_BASE, Store, bot_stream};

async fn fresh_store(name: &str) -> Result<Store, String> {
    let dir = test_harness::test_dir(name)?;
    let opened = Store::open(&dir, 1).await.map_err(|e| e.to_string())?;
    Ok(opened.store)
}

fn sample_intent(stream: &str, record_id: &str, task_id: &str, turn: i64) -> JournalEntry {
    JournalEntry {
        stream: stream.to_owned(),
        record_id: record_id.to_owned(),
        kind: "tool_intent".to_owned(),
        content: json!({"tool": "shell", "masked": true}),
        local_detail: Some("{\"raw\":\"secret\"}".to_owned()),
        created_at_ms: 1_700_000_000_001,
        task_id: Some(task_id.to_owned()),
        turn: Some(turn),
        effect: Some(ToolEffect::Irreversible),
        offline_at_create: false,
    }
}

fn sample_row(
    stream: &str,
    record_id: &str,
    kind: &str,
    turn: Option<i64>,
    local_detail: Option<&str>,
) -> JournalEntry {
    JournalEntry {
        stream: stream.to_owned(),
        record_id: record_id.to_owned(),
        kind: kind.to_owned(),
        content: json!({"kind": kind}),
        local_detail: local_detail.map(ToOwned::to_owned),
        created_at_ms: 1_700_000_000_002,
        task_id: Some("task-1".to_owned()),
        turn,
        effect: None,
        offline_at_create: false,
    }
}

#[tokio::test]
async fn journal_row_is_outbox_row() -> Result<(), String> {
    let store = fresh_store("store_journal_outbox_row").await?;
    let sync_mode: i64 = sqlx::query_scalar("PRAGMA synchronous")
        .fetch_one(store.pool())
        .await
        .map_err(|e| e.to_string())?;
    assert_eq!(sync_mode, 2);

    let stream = bot_stream("b1");
    let before = store
        .outbox_pending(&stream, 100)
        .await
        .map_err(|e| e.to_string())?;
    assert!(before.is_empty());

    let entry = sample_intent(&stream, "r-1", "task-1", 1);
    let written = store
        .journal_append(&entry)
        .await
        .map_err(|e| e.to_string())?;
    assert_eq!(written.effect, Some(ToolEffect::Irreversible));
    assert_eq!(written.turn, Some(1));

    let after = store
        .outbox_pending(&stream, 100)
        .await
        .map_err(|e| e.to_string())?;
    assert_eq!(after.len(), 1);
    let rec_rows = store.records(&stream).await.map_err(|e| e.to_string())?;
    assert!(rec_rows.is_empty());
    Ok(())
}

#[tokio::test]
async fn journal_trim_after_normal_end() -> Result<(), String> {
    let store = fresh_store("store_journal_trim").await?;
    let stream = bot_stream("b1");
    for entry in [
        sample_intent(&stream, "r-intent", "task-1", 1),
        sample_row(
            &stream,
            "r-steer",
            "user_said",
            None,
            Some("{\"text\":\"steer\"}"),
        ),
        sample_row(&stream, "r-end", "turn_ended", Some(1), Some("{\"ok\":1}")),
    ] {
        store
            .journal_append(&entry)
            .await
            .map_err(|e| e.to_string())?;
    }
    store
        .journal_trim("task-1", 1)
        .await
        .map_err(|e| e.to_string())?;
    let rows = store
        .journal_task_rows("task-1")
        .await
        .map_err(|e| e.to_string())?;
    assert_eq!(rows.len(), 3);
    for row in &rows {
        if row.turn.is_some() {
            assert_eq!(row.local_detail, None);
        } else {
            assert_eq!(row.local_detail.as_deref(), Some("{\"text\":\"steer\"}"));
        }
    }
    Ok(())
}

#[tokio::test]
async fn journal_open_lists_only_turns_without_turn_ended() -> Result<(), String> {
    let store = fresh_store("store_journal_open").await?;
    let stream = bot_stream("b1");
    let turn1 = sample_row(&stream, "r-t1-start", "turn_started", Some(1), None);
    store
        .journal_append(&turn1)
        .await
        .map_err(|e| e.to_string())?;
    let open = store.journal_open().await.map_err(|e| e.to_string())?;
    assert_eq!(open.len(), 1);

    let end1 = sample_row(&stream, "r-t1-end", "turn_ended", Some(1), None);
    store
        .journal_append(&end1)
        .await
        .map_err(|e| e.to_string())?;
    let after = store.journal_open().await.map_err(|e| e.to_string())?;
    assert!(after.is_empty());
    Ok(())
}

#[tokio::test]
async fn journal_append_unless_halted_stops_on_halt_row() -> Result<(), String> {
    let store = fresh_store("store_journal_halt").await?;
    let stream = bot_stream("b1");
    let start = sample_row(&stream, "r-start", "turn_started", Some(1), None);
    let start_row = store
        .journal_append(&start)
        .await
        .map_err(|e| e.to_string())?;
    let started_seq = OUTBOX_SEQ_BASE.saturating_add(start_row.local_id);

    let stopped = sample_row(&stream, "r-stop", "task_stopped", None, None);
    store
        .journal_append(&stopped)
        .await
        .map_err(|e| e.to_string())?;

    let intent = sample_intent(&stream, "r-intent", "task-1", 1);
    let outcome = store
        .journal_append_unless_halted(&intent, started_seq, "task-1")
        .await
        .map_err(|e| e.to_string())?;
    assert_eq!(outcome, JournalAppend::Stopped);
    Ok(())
}
