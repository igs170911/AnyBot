use sqlx::{Postgres, Transaction};

#[derive(Debug, Clone)]
pub struct SettingRow {
    pub seq: i64,
    pub device_id: String,
    pub content: String,
    pub received_ms: Option<i64>,
}

pub async fn setting_rows_in(
    tx: &mut Transaction<'_, Postgres>,
    stream_id: &str,
) -> Result<Vec<SettingRow>, sqlx::Error> {
    let rows = sqlx::query!(
        "SELECT seq, device_id, content, received_ms FROM records \
         WHERE stream_id = $1 AND kind = 'setting' ORDER BY seq",
        stream_id,
    )
    .fetch_all(&mut **tx)
    .await?;
    Ok(rows
        .into_iter()
        .map(|r| SettingRow {
            seq: r.seq,
            device_id: r.device_id,
            content: r.content,
            received_ms: r.received_ms,
        })
        .collect())
}

pub async fn set_timing_in(
    tx: &mut Transaction<'_, Postgres>,
    stream_id: &str,
    seq: i64,
    received_ms: i64,
    corrected_ms: i64,
) -> Result<(), sqlx::Error> {
    sqlx::query!(
        "UPDATE records SET received_ms = $3, corrected_ms = $4 \
         WHERE stream_id = $1 AND seq = $2",
        stream_id,
        seq,
        received_ms,
        corrected_ms,
    )
    .execute(&mut **tx)
    .await?;
    Ok(())
}

pub async fn rename_bot_in(
    tx: &mut Transaction<'_, Postgres>,
    id: &str,
    name: &str,
) -> Result<(), sqlx::Error> {
    sqlx::query!("UPDATE bots SET name = $2 WHERE id = $1", id, name)
        .execute(&mut **tx)
        .await?;
    Ok(())
}

pub async fn setting_rows(
    pool: &crate::Pool,
    stream_id: &str,
) -> Result<Vec<SettingRow>, sqlx::Error> {
    let mut tx = pool.begin().await?;
    let rows = setting_rows_in(&mut tx, stream_id).await?;
    tx.commit().await?;
    Ok(rows)
}
