use sqlx::{Postgres, Transaction};

use crate::Pool;
use crate::streams::{RecordFields, RecordRow, record_from_row};

pub async fn begin(pool: &Pool) -> Result<Transaction<'_, Postgres>, sqlx::Error> {
    pool.begin().await
}

pub async fn lock_max_seq(
    tx: &mut Transaction<'_, Postgres>,
    stream_id: &str,
) -> Result<i64, sqlx::Error> {
    let row = sqlx::query!(
        "SELECT next_seq FROM record_streams WHERE id = $1 FOR UPDATE",
        stream_id,
    )
    .fetch_optional(&mut **tx)
    .await?;
    let stream = row.ok_or(sqlx::Error::Protocol("stream row missing".into()))?;
    stream
        .next_seq
        .checked_sub(1)
        .ok_or(sqlx::Error::Protocol("next_seq below one".into()))
}

pub async fn record_at_in(
    tx: &mut Transaction<'_, Postgres>,
    stream_id: &str,
    seq: i64,
) -> Result<Option<RecordRow>, sqlx::Error> {
    let row = sqlx::query_as!(
        RecordFields,
        "SELECT stream_id, seq, device_id, kind, content, created_at_ms, offline_sent, \
         task_id, task_seq, purged FROM records WHERE stream_id = $1 AND seq = $2",
        stream_id,
        seq,
    )
    .fetch_optional(&mut **tx)
    .await?;
    Ok(row.map(record_from_row))
}

pub async fn chat_rows_between_in(
    tx: &mut Transaction<'_, Postgres>,
    stream_id: &str,
    low: i64,
    high: i64,
) -> Result<Vec<RecordRow>, sqlx::Error> {
    let rows = sqlx::query_as!(
        RecordFields,
        "SELECT stream_id, seq, device_id, kind, content, created_at_ms, offline_sent, \
         task_id, task_seq, purged FROM records \
         WHERE stream_id = $1 AND seq >= $2 AND seq <= $3 AND purged = FALSE \
         AND kind IN ('message', 'model_answer') ORDER BY seq",
        stream_id,
        low,
        high,
    )
    .fetch_all(&mut **tx)
    .await?;
    Ok(rows.into_iter().map(record_from_row).collect())
}

pub async fn mark_purged_in(
    tx: &mut Transaction<'_, Postgres>,
    stream_id: &str,
    seq: i64,
    content: &str,
) -> Result<(), sqlx::Error> {
    sqlx::query!(
        "UPDATE records SET content = $3, purged = TRUE WHERE stream_id = $1 AND seq = $2",
        stream_id,
        seq,
        content,
    )
    .execute(&mut **tx)
    .await?;
    Ok(())
}
