use crate::Pool;

#[derive(Debug)]
pub struct IdempotencyRow {
    pub scope: String,
    pub request_hash: String,
    pub status_code: i32,
    pub body: String,
    pub at_ms: i64,
}

pub async fn find_idempotency(
    pool: &Pool,
    key: &str,
    scope: &str,
) -> Result<Option<IdempotencyRow>, sqlx::Error> {
    let row = sqlx::query!(
        "SELECT scope, request_hash, status_code, body, at_ms FROM idempotency \
         WHERE key = $1 AND scope = $2",
        key,
        scope,
    )
    .fetch_optional(pool)
    .await?;
    let Some(row) = row else {
        return Ok(None);
    };
    Ok(Some(IdempotencyRow {
        scope: row.scope,
        request_hash: row.request_hash,
        status_code: row.status_code,
        body: row.body,
        at_ms: row.at_ms,
    }))
}

pub async fn insert_idempotency(
    pool: &Pool,
    key: &str,
    scope: &str,
    request_hash: &str,
    status_code: i32,
    body: &str,
    at_ms: i64,
) -> Result<(), sqlx::Error> {
    sqlx::query!(
        "INSERT INTO idempotency (key, scope, request_hash, status_code, body, at_ms) \
         VALUES ($1, $2, $3, $4, $5, $6) \
         ON CONFLICT (key, scope) DO NOTHING",
        key,
        scope,
        request_hash,
        status_code,
        body,
        at_ms,
    )
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn prune_idempotency_before(pool: &Pool, floor_ms: i64) -> Result<u64, sqlx::Error> {
    let result = sqlx::query!("DELETE FROM idempotency WHERE at_ms < $1", floor_ms)
        .execute(pool)
        .await?;
    Ok(result.rows_affected())
}

pub async fn count_idempotency_key(pool: &Pool, key: &str) -> Result<i64, sqlx::Error> {
    let row = sqlx::query!(
        "SELECT count(*) AS total FROM idempotency WHERE key = $1",
        key,
    )
    .fetch_one(pool)
    .await?;
    row.total
        .ok_or(sqlx::Error::ColumnNotFound("total".to_owned()))
}
