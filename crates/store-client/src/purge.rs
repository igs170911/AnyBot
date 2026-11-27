use anybot_domain::record_actions::{
    KIND_RECORD_CLEAR, KIND_RECORD_DELETE, KIND_RECORD_PURGE_PERMANENT, is_chat_kind,
    scrub_chat_content,
};
use anybot_protocol::StoredRecord;
use serde_json::Value;
use sqlx::sqlite::SqliteConnection;

use crate::error::StoreError;

pub(crate) async fn apply_purge(
    conn: &mut SqliteConnection,
    stream: &str,
    purge: &StoredRecord,
) -> Result<(), StoreError> {
    if purge.kind != KIND_RECORD_PURGE_PERMANENT {
        return Ok(());
    }
    let Some(target_seq) = purge.content.get("target_seq").and_then(Value::as_i64) else {
        return Ok(());
    };
    let Some((low, high)) = hidden_range(conn, stream, target_seq).await? else {
        return Ok(());
    };
    let rows = sqlx::query!(
        "SELECT seq, kind, content FROM records WHERE stream = ? AND seq BETWEEN ? AND ?",
        stream,
        low,
        high
    )
    .fetch_all(&mut *conn)
    .await?;
    for row in rows {
        if !is_chat_kind(&row.kind) {
            continue;
        }
        let content: Value = serde_json::from_str(&row.content)
            .map_err(|e| StoreError::BadValue(format!("record content: {e}")))?;
        let scrubbed = scrub_chat_content(&row.kind, &content).to_string();
        sqlx::query!(
            "UPDATE records SET content = ?, purged = 1 WHERE stream = ? AND seq = ?",
            scrubbed,
            stream,
            row.seq
        )
        .execute(&mut *conn)
        .await?;
    }
    Ok(())
}

async fn hidden_range(
    conn: &mut SqliteConnection,
    stream: &str,
    target_seq: i64,
) -> Result<Option<(i64, i64)>, StoreError> {
    let row = sqlx::query!(
        "SELECT kind, content FROM records WHERE stream = ? AND seq = ?",
        stream,
        target_seq
    )
    .fetch_optional(&mut *conn)
    .await?;
    let Some(row) = row else {
        return Ok(None);
    };
    let content: Value = serde_json::from_str(&row.content)
        .map_err(|e| StoreError::BadValue(format!("record content: {e}")))?;
    if row.kind == KIND_RECORD_DELETE {
        return Ok(content
            .get("target_seq")
            .and_then(Value::as_i64)
            .map(|seq| (seq, seq)));
    }
    if row.kind == KIND_RECORD_CLEAR {
        return Ok(content
            .get("through_seq")
            .and_then(Value::as_i64)
            .map(|through| (1, through)));
    }
    Ok(None)
}
