use anybot_protocol::StoredRecord;
use serde_json::{Value, json};

use crate::{ACCOUNT_STREAM, NewOutbox, OutboxState, Store, bot_stream};

mod journal;
mod recovery;

fn stored(seq: i64, kind: &str, content: Value) -> StoredRecord {
    StoredRecord {
        seq,
        device_id: format!("0190aaaa-bbbb-7ccc-8ddd-{seq:012}"),
        kind: kind.to_owned(),
        content,
        created_at_ms: 1_700_000_000_000_i64.saturating_add(seq),
        offline_sent: false,
        task_id: None,
        task_seq: None,
    }
}

fn pending(record_id: &str, kind: &str) -> NewOutbox {
    NewOutbox {
        stream: ACCOUNT_STREAM.to_owned(),
        record_id: record_id.to_owned(),
        kind: kind.to_owned(),
        content: json!({"n": record_id}),
        created_at_ms: 1_700_000_000_000,
        task_id: None,
        offline_at_create: false,
    }
}

async fn fresh(name: &str) -> Result<Store, String> {
    let dir = test_harness::test_dir(name)?;
    let opened = Store::open(&dir, 1)
        .await
        .map_err(|e| format!("open failed: {e}"))?;
    Ok(opened.store)
}

#[tokio::test]
async fn contiguous_page_advances_progress_and_keeps_rows() -> Result<(), String> {
    let store = fresh("store_client_contiguous").await?;
    let page = vec![
        stored(1, "message", json!({"t": 1})),
        stored(2, "message", json!({"t": 2})),
    ];
    let outcome = store
        .apply_page("bot/b1", &page)
        .await
        .map_err(|e| e.to_string())?;
    assert_eq!(outcome.inserted, 2);
    assert_eq!(outcome.through_seq, 2);
    assert!(!outcome.gap);
    let progress = store.progress("bot/b1").await.map_err(|e| e.to_string())?;
    assert_eq!(
        (progress.seq, progress.record_id.as_str()),
        (2, "0190aaaa-bbbb-7ccc-8ddd-000000000002")
    );
    let rows = store.records("bot/b1").await.map_err(|e| e.to_string())?;
    assert_eq!(rows.len(), 2);
    Ok(())
}

#[tokio::test]
async fn duplicate_and_shuffled_records_are_ignored_or_ordered() -> Result<(), String> {
    let store = fresh("store_client_duplicates").await?;
    let first = vec![
        stored(2, "message", json!(2)),
        stored(1, "message", json!(1)),
    ];
    store
        .apply_page("bot/b1", &first)
        .await
        .map_err(|e| e.to_string())?;
    let again = store
        .apply_page("bot/b1", &first)
        .await
        .map_err(|e| e.to_string())?;
    assert_eq!(again.inserted, 0);
    assert_eq!(again.through_seq, 2);
    Ok(())
}

#[tokio::test]
async fn missing_record_stops_at_the_gap() -> Result<(), String> {
    let store = fresh("store_client_gap").await?;
    let page = vec![
        stored(1, "message", json!(1)),
        stored(3, "message", json!(3)),
    ];
    let outcome = store
        .apply_page("bot/b1", &page)
        .await
        .map_err(|e| e.to_string())?;
    assert!(outcome.gap);
    assert_eq!(outcome.through_seq, 1);
    let progress = store.progress("bot/b1").await.map_err(|e| e.to_string())?;
    assert_eq!(progress.seq, 1);
    Ok(())
}

#[tokio::test]
async fn permanent_purge_scrubs_the_hidden_chat_content() -> Result<(), String> {
    let store = fresh("store_client_purge").await?;
    let page = vec![
        stored(1, "message", json!({"text": "secret"})),
        stored(
            2,
            "model_answer",
            json!({"text": "answer", "thinking": "idea", "model": "m"}),
        ),
        stored(3, "record_purge", json!({"through_seq": 2})),
        stored(4, "record_purge_permanent", json!({"target_seq": 3})),
    ];
    let stream = bot_stream("b1");
    store
        .apply_page(&stream, &page)
        .await
        .map_err(|e| e.to_string())?;
    let rows = store.records(&stream).await.map_err(|e| e.to_string())?;
    let first = rows.first().ok_or("missing row 1")?;
    assert_eq!(first.content, Value::Null);
    assert!(first.purged);
    let second = rows.get(1).ok_or("missing row 2")?;
    assert_eq!(second.content.get("text"), Some(&Value::Null));
    assert_eq!(second.content.get("model"), Some(&json!("m")));
    Ok(())
}

#[tokio::test]
async fn replace_stream_rewrites_rows_and_progress() -> Result<(), String> {
    let store = fresh("store_client_replace").await?;
    let old = vec![
        stored(1, "message", json!(1)),
        stored(2, "message", json!(2)),
    ];
    store
        .apply_page("bot/b1", &old)
        .await
        .map_err(|e| e.to_string())?;
    let fresh_rows = vec![stored(1, "message", json!("new"))];
    store
        .replace_stream("bot/b1", &fresh_rows)
        .await
        .map_err(|e| e.to_string())?;
    let rows = store.records("bot/b1").await.map_err(|e| e.to_string())?;
    assert_eq!(rows.len(), 1);
    let progress = store.progress("bot/b1").await.map_err(|e| e.to_string())?;
    assert_eq!(progress.seq, 1);
    let skipped = vec![stored(2, "message", json!(2))];
    assert!(store.replace_stream("bot/b1", &skipped).await.is_err());
    Ok(())
}

#[tokio::test]
async fn outbox_keeps_order_and_tracks_attempt_accept_and_reject() -> Result<(), String> {
    let store = fresh("store_client_outbox").await?;
    for id in ["a", "b", "c"] {
        store
            .enqueue(&pending(id, "setting"))
            .await
            .map_err(|e| e.to_string())?;
    }
    let listed = store
        .outbox_pending(ACCOUNT_STREAM, 10)
        .await
        .map_err(|e| e.to_string())?;
    let ids: Vec<&str> = listed.iter().map(|row| row.record_id.as_str()).collect();
    assert_eq!(ids, ["a", "b", "c"]);
    let first = listed.first().ok_or("missing a")?;
    store
        .outbox_mark_attempt(first.local_id, 5, true, &json!({"n": "a", "x": 1}))
        .await
        .map_err(|e| e.to_string())?;
    assert!(
        store
            .outbox_cancel_unsent("a")
            .await
            .map_err(|e| e.to_string())?
            .is_none()
    );
    assert!(
        store
            .outbox_cancel_unsent("b")
            .await
            .map_err(|e| e.to_string())?
            .is_some()
    );
    store
        .outbox_reject(first.local_id, "bad-thing", 100)
        .await
        .map_err(|e| e.to_string())?;
    assert_eq!(
        store.outbox_counts().await.map_err(|e| e.to_string())?,
        (1, 1)
    );
    let all = store.outbox_all().await.map_err(|e| e.to_string())?;
    let rejected = all.first().ok_or("missing rejected")?;
    assert_eq!(rejected.state, OutboxState::Rejected);
    assert_eq!(rejected.offline_sent, Some(true));
    assert_eq!(
        store
            .outbox_drop_expired(99)
            .await
            .map_err(|e| e.to_string())?,
        0
    );
    assert_eq!(
        store
            .outbox_drop_expired(100)
            .await
            .map_err(|e| e.to_string())?,
        1
    );
    Ok(())
}

#[tokio::test]
async fn pending_outbox_survives_reopening_the_file() -> Result<(), String> {
    let dir = test_harness::test_dir("store_client_durable")?;
    let opened = Store::open(&dir, 1).await.map_err(|e| e.to_string())?;
    opened
        .store
        .enqueue(&pending("keep", "message"))
        .await
        .map_err(|e| e.to_string())?;
    opened.store.close().await;
    let reopened = Store::open(&dir, 2).await.map_err(|e| e.to_string())?;
    assert!(reopened.recovery.is_none());
    let rows = reopened
        .store
        .outbox_all()
        .await
        .map_err(|e| e.to_string())?;
    assert_eq!(rows.len(), 1);
    Ok(())
}

#[tokio::test]
async fn settings_merge_replica_and_pending_edits() -> Result<(), String> {
    let store = fresh("store_client_settings").await?;
    let page = vec![stored(
        1,
        "setting",
        json!({"field": "theme", "value": "dark", "changed_at_ms": 1000, "offset_ms": 0}),
    )];
    store
        .apply_page(ACCOUNT_STREAM, &page)
        .await
        .map_err(|e| e.to_string())?;
    let mut edit = pending("0190aaaa-bbbb-7ccc-8ddd-eeeeffff0001", "setting");
    edit.content =
        json!({"field": "theme", "value": "light", "changed_at_ms": 2000, "offset_ms": 0});
    store.enqueue(&edit).await.map_err(|e| e.to_string())?;
    let merged = store.settings().await.map_err(|e| e.to_string())?;
    let theme = merged.first().ok_or("missing theme")?;
    assert_eq!(theme.value, "light");
    Ok(())
}
