use crate::Pool;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CodeRow {
    pub id: String,
    pub account: String,
    pub expires_at_ms: i64,
    pub used_at_ms: Option<i64>,
    pub redeemed_device: Option<String>,
    pub redeemed_address: Option<String>,
}

pub struct NewCode<'a> {
    pub id: &'a str,
    pub account: &'a str,
    pub algorithm: &'a str,
    pub digest: &'a [u8],
    pub created_at_ms: i64,
    pub expires_at_ms: i64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InsertOutcome {
    Stored,
    Full,
    Duplicate,
}

pub async fn insert_code(
    pool: &Pool,
    new: &NewCode<'_>,
    cap: i64,
) -> Result<InsertOutcome, sqlx::Error> {
    let mut tx = pool.begin().await?;
    sqlx::query!("SELECT pg_advisory_xact_lock(hashtext('pairing-codes|cap'))")
        .execute(&mut *tx)
        .await?;
    if pending_in(&mut tx, new.created_at_ms).await? >= cap {
        tx.rollback().await?;
        return Ok(InsertOutcome::Full);
    }
    sqlx::query!(
        "DELETE FROM pairing_codes WHERE algorithm = $1 AND digest = $2 AND expires_at_ms <= $3",
        new.algorithm,
        new.digest,
        new.created_at_ms,
    )
    .execute(&mut *tx)
    .await?;
    if insert_in(&mut tx, new).await? {
        tx.commit().await?;
        return Ok(InsertOutcome::Stored);
    }
    tx.rollback().await?;
    Ok(InsertOutcome::Duplicate)
}

async fn pending_in(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    now_ms: i64,
) -> Result<i64, sqlx::Error> {
    let row = sqlx::query!(
        "SELECT count(*) AS total FROM pairing_codes WHERE used_at_ms IS NULL AND expires_at_ms > $1",
        now_ms,
    )
    .fetch_one(&mut **tx)
    .await?;
    row.total
        .ok_or(sqlx::Error::ColumnNotFound("total".to_owned()))
}

async fn insert_in(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    new: &NewCode<'_>,
) -> Result<bool, sqlx::Error> {
    let inserted = sqlx::query!(
        "INSERT INTO pairing_codes (id, account, algorithm, digest, created_at_ms, expires_at_ms) \
         VALUES ($1, $2, $3, $4, $5, $6) ON CONFLICT (algorithm, digest) DO NOTHING",
        new.id,
        new.account,
        new.algorithm,
        new.digest,
        new.created_at_ms,
        new.expires_at_ms,
    )
    .execute(&mut **tx)
    .await?;
    Ok(inserted.rows_affected() == 1)
}

pub async fn find_by_hash(
    pool: &Pool,
    algorithm: &str,
    digest: &[u8],
) -> Result<Option<CodeRow>, sqlx::Error> {
    let row = sqlx::query_as!(
        CodeRow,
        "SELECT id, account, expires_at_ms, used_at_ms, redeemed_device, redeemed_address \
         FROM pairing_codes WHERE algorithm = $1 AND digest = $2",
        algorithm,
        digest,
    )
    .fetch_optional(pool)
    .await?;
    Ok(row)
}

pub async fn claim(pool: &Pool, id: &str, address: &str, now_ms: i64) -> Result<bool, sqlx::Error> {
    let done = sqlx::query!(
        "UPDATE pairing_codes SET used_at_ms = $3, redeemed_address = $2 \
         WHERE id = $1 AND used_at_ms IS NULL AND expires_at_ms > $3",
        id,
        address,
        now_ms,
    )
    .execute(pool)
    .await?;
    Ok(done.rows_affected() == 1)
}

pub async fn set_redeemed_device(
    pool: &Pool,
    id: &str,
    device_id: &str,
) -> Result<(), sqlx::Error> {
    sqlx::query!(
        "UPDATE pairing_codes SET redeemed_device = $2 WHERE id = $1",
        id,
        device_id,
    )
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn record_redeem(
    pool: &Pool,
    key: &str,
    address: &str,
    code_id: &str,
    device_id: &str,
    at_ms: i64,
) -> Result<(), sqlx::Error> {
    sqlx::query!(
        "INSERT INTO pairing_redeems (key, address, code_id, device_id, at_ms) \
         VALUES ($1, $2, $3, $4, $5) ON CONFLICT (key, address) DO NOTHING",
        key,
        address,
        code_id,
        device_id,
        at_ms,
    )
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn redeem_of(
    pool: &Pool,
    key: &str,
    address: &str,
) -> Result<Option<(String, String)>, sqlx::Error> {
    let row = sqlx::query!(
        "SELECT code_id, device_id FROM pairing_redeems WHERE key = $1 AND address = $2",
        key,
        address,
    )
    .fetch_optional(pool)
    .await?;
    Ok(row.map(|row| (row.code_id, row.device_id)))
}

pub async fn prune_expired(pool: &Pool, now_ms: i64) -> Result<(), sqlx::Error> {
    sqlx::query!(
        "DELETE FROM pairing_codes WHERE expires_at_ms <= $1",
        now_ms
    )
    .execute(pool)
    .await?;
    sqlx::query!("DELETE FROM pairing_redeems WHERE code_id NOT IN (SELECT id FROM pairing_codes)")
        .execute(pool)
        .await?;
    Ok(())
}
