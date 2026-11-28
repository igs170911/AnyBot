use crate::Pool;
use crate::streams::{RecordFields, RecordRow, record_from_row};

pub async fn list_task_records(
    pool: &Pool,
    stream_id: &str,
    task_id: &str,
) -> Result<Vec<RecordRow>, sqlx::Error> {
    let rows = sqlx::query_as!(
        RecordFields,
        "SELECT stream_id, seq, device_id, kind, content, created_at_ms, offline_sent, \
         task_id, task_seq, purged FROM records \
         WHERE stream_id = $1 AND task_id = $2 AND NOT purged ORDER BY task_seq",
        stream_id,
        task_id,
    )
    .fetch_all(pool)
    .await?;
    Ok(rows.into_iter().map(record_from_row).collect())
}

pub async fn answer_record_exists(
    pool: &Pool,
    stream_id: &str,
    task_id: &str,
    answer_seq: i64,
    answer_kind: &str,
) -> Result<bool, sqlx::Error> {
    let row = sqlx::query!(
        "SELECT kind, task_id, purged FROM records \
         WHERE stream_id = $1 AND seq = $2",
        stream_id,
        answer_seq,
    )
    .fetch_optional(pool)
    .await?;
    let Some(row) = row else {
        return Ok(false);
    };
    Ok(!row.purged && row.kind == answer_kind && row.task_id.as_deref() == Some(task_id))
}
