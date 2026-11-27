use sqlx::{Postgres, Transaction};

pub async fn ensure_stream_in(
    tx: &mut Transaction<'_, Postgres>,
    id: &str,
    kind: &str,
    owner: &str,
    bot_id: Option<&str>,
    now_ms: i64,
) -> Result<(), sqlx::Error> {
    sqlx::query!(
        "INSERT INTO record_streams (id, kind, owner, bot_id, created_at_ms) \
         VALUES ($1, $2, $3, $4, $5) ON CONFLICT (id) DO NOTHING",
        id,
        kind,
        owner,
        bot_id,
        now_ms,
    )
    .execute(&mut **tx)
    .await?;
    Ok(())
}

pub async fn ensure_bot_in(
    tx: &mut Transaction<'_, Postgres>,
    id: &str,
    account: &str,
    name: &str,
    created_at_ms: i64,
) -> Result<(), sqlx::Error> {
    sqlx::query!(
        "INSERT INTO bots (id, account, name, created_at_ms) VALUES ($1, $2, $3, $4) \
         ON CONFLICT (id) DO NOTHING",
        id,
        account,
        name,
        created_at_ms,
    )
    .execute(&mut **tx)
    .await?;
    Ok(())
}

pub async fn set_owned_bot_deleted_in(
    tx: &mut Transaction<'_, Postgres>,
    id: &str,
    account: &str,
    deleted: bool,
) -> Result<u64, sqlx::Error> {
    let done = sqlx::query!(
        "UPDATE bots SET deleted = $3 WHERE id = $1 AND account = $2",
        id,
        account,
        deleted,
    )
    .execute(&mut **tx)
    .await?;
    Ok(done.rows_affected())
}
