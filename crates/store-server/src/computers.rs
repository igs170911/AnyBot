use sqlx::{Postgres, Transaction};

use crate::Pool;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ComputerRow {
    pub device_id: String,
    pub account: String,
    pub name: String,
    pub key_hash: String,
    pub paired: bool,
    pub delegate_enabled: bool,
    pub created_at_ms: i64,
    pub last_seen_ms: i64,
    pub password_entered_ms: i64,
    pub credential_used_ms: Option<i64>,
    pub online: bool,
    pub offline_at_ms: Option<i64>,
    pub offline_reason: Option<String>,
    pub removed_at_ms: Option<i64>,
}

pub struct NewComputer<'a> {
    pub device_id: &'a str,
    pub account: &'a str,
    pub name: &'a str,
    pub key_hash: &'a str,
    pub paired: bool,
    pub now_ms: i64,
}

pub async fn lock_account_in(
    tx: &mut Transaction<'_, Postgres>,
    account: &str,
) -> Result<(), sqlx::Error> {
    let key = format!("computers|{account}");
    sqlx::query!("SELECT pg_advisory_xact_lock(hashtext($1))", key)
        .execute(&mut **tx)
        .await?;
    Ok(())
}

pub async fn insert_in(
    tx: &mut Transaction<'_, Postgres>,
    new: &NewComputer<'_>,
) -> Result<(), sqlx::Error> {
    sqlx::query!(
        "INSERT INTO computers (device_id, account, name, key_hash, paired, created_at_ms, \
         last_seen_ms, password_entered_ms) VALUES ($1, $2, $3, $4, $5, $6, $6, $6)",
        new.device_id,
        new.account,
        new.name,
        new.key_hash,
        new.paired,
        new.now_ms,
    )
    .execute(&mut **tx)
    .await?;
    Ok(())
}

pub async fn find_by_key_hash(
    pool: &Pool,
    key_hash: &str,
) -> Result<Option<ComputerRow>, sqlx::Error> {
    let row = sqlx::query_as!(
        ComputerRow,
        "SELECT device_id, account, name, key_hash, paired, delegate_enabled, created_at_ms, \
         last_seen_ms, password_entered_ms, credential_used_ms, online, offline_at_ms, \
         offline_reason, removed_at_ms FROM computers WHERE key_hash = $1",
        key_hash,
    )
    .fetch_optional(pool)
    .await?;
    Ok(row)
}

pub async fn find_by_device(
    pool: &Pool,
    device_id: &str,
) -> Result<Option<ComputerRow>, sqlx::Error> {
    let row = sqlx::query_as!(
        ComputerRow,
        "SELECT device_id, account, name, key_hash, paired, delegate_enabled, created_at_ms, \
         last_seen_ms, password_entered_ms, credential_used_ms, online, offline_at_ms, \
         offline_reason, removed_at_ms FROM computers WHERE device_id = $1",
        device_id,
    )
    .fetch_optional(pool)
    .await?;
    Ok(row)
}

pub async fn list_live_in(
    tx: &mut Transaction<'_, Postgres>,
    account: &str,
) -> Result<Vec<ComputerRow>, sqlx::Error> {
    let rows = sqlx::query_as!(
        ComputerRow,
        "SELECT device_id, account, name, key_hash, paired, delegate_enabled, created_at_ms, \
         last_seen_ms, password_entered_ms, credential_used_ms, online, offline_at_ms, \
         offline_reason, removed_at_ms FROM computers \
         WHERE account = $1 AND removed_at_ms IS NULL ORDER BY created_at_ms, device_id",
        account,
    )
    .fetch_all(&mut **tx)
    .await?;
    Ok(rows)
}

pub async fn list_live(pool: &Pool, account: &str) -> Result<Vec<ComputerRow>, sqlx::Error> {
    let mut tx = pool.begin().await?;
    let rows = list_live_in(&mut tx, account).await?;
    tx.commit().await?;
    Ok(rows)
}

pub async fn list_all_live(pool: &Pool) -> Result<Vec<ComputerRow>, sqlx::Error> {
    let rows = sqlx::query_as!(
        ComputerRow,
        "SELECT device_id, account, name, key_hash, paired, delegate_enabled, created_at_ms, \
         last_seen_ms, password_entered_ms, credential_used_ms, online, offline_at_ms, \
         offline_reason, removed_at_ms FROM computers WHERE removed_at_ms IS NULL",
    )
    .fetch_all(pool)
    .await?;
    Ok(rows)
}

pub async fn mark_removed_in(
    tx: &mut Transaction<'_, Postgres>,
    device_id: &str,
    now_ms: i64,
    reason: &str,
) -> Result<bool, sqlx::Error> {
    let done = sqlx::query!(
        "UPDATE computers SET removed_at_ms = $2, online = FALSE, \
         offline_at_ms = COALESCE(offline_at_ms, $2), offline_reason = $3 \
         WHERE device_id = $1 AND removed_at_ms IS NULL",
        device_id,
        now_ms,
        reason,
    )
    .execute(&mut **tx)
    .await?;
    Ok(done.rows_affected() == 1)
}

pub async fn rename_in(
    tx: &mut Transaction<'_, Postgres>,
    device_id: &str,
    name: &str,
) -> Result<(), sqlx::Error> {
    sqlx::query!(
        "UPDATE computers SET name = $2 WHERE device_id = $1",
        device_id,
        name,
    )
    .execute(&mut **tx)
    .await?;
    Ok(())
}

pub async fn set_delegate(
    pool: &Pool,
    device_id: &str,
    enabled: bool,
) -> Result<bool, sqlx::Error> {
    let done = sqlx::query!(
        "UPDATE computers SET delegate_enabled = $2 \
         WHERE device_id = $1 AND removed_at_ms IS NULL AND delegate_enabled <> $2",
        device_id,
        enabled,
    )
    .execute(pool)
    .await?;
    Ok(done.rows_affected() == 1)
}

pub async fn touch_seen(pool: &Pool, device_id: &str, seen_ms: i64) -> Result<(), sqlx::Error> {
    sqlx::query!(
        "UPDATE computers SET last_seen_ms = $2 WHERE device_id = $1 AND last_seen_ms < $2",
        device_id,
        seen_ms,
    )
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn mark_online(pool: &Pool, device_id: &str, seen_ms: i64) -> Result<bool, sqlx::Error> {
    let done = sqlx::query!(
        "UPDATE computers SET online = TRUE, offline_at_ms = NULL, offline_reason = NULL, \
         last_seen_ms = GREATEST(last_seen_ms, $2) \
         WHERE device_id = $1 AND online = FALSE AND removed_at_ms IS NULL",
        device_id,
        seen_ms,
    )
    .execute(pool)
    .await?;
    Ok(done.rows_affected() == 1)
}

pub async fn mark_offline(
    pool: &Pool,
    device_id: &str,
    at_ms: i64,
    reason: Option<&str>,
) -> Result<bool, sqlx::Error> {
    let done = sqlx::query!(
        "UPDATE computers SET online = FALSE, offline_at_ms = $2, offline_reason = $3, \
         last_seen_ms = GREATEST(last_seen_ms, $2) \
         WHERE device_id = $1 AND online = TRUE AND removed_at_ms IS NULL \
         AND last_seen_ms <= $2",
        device_id,
        at_ms,
        reason,
    )
    .execute(pool)
    .await?;
    Ok(done.rows_affected() == 1)
}

pub async fn mark_credential_used(
    pool: &Pool,
    device_id: &str,
    now_ms: i64,
) -> Result<(), sqlx::Error> {
    sqlx::query!(
        "UPDATE computers SET credential_used_ms = $2 \
         WHERE device_id = $1 AND credential_used_ms IS NULL",
        device_id,
        now_ms,
    )
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn replace_unused_key_in(
    tx: &mut Transaction<'_, Postgres>,
    device_id: &str,
    key_hash: &str,
) -> Result<bool, sqlx::Error> {
    let done = sqlx::query!(
        "UPDATE computers SET key_hash = $2 \
         WHERE device_id = $1 AND credential_used_ms IS NULL AND removed_at_ms IS NULL",
        device_id,
        key_hash,
    )
    .execute(&mut **tx)
    .await?;
    Ok(done.rows_affected() == 1)
}

pub async fn enter_password(pool: &Pool, device_id: &str, now_ms: i64) -> Result<(), sqlx::Error> {
    sqlx::query!(
        "UPDATE computers SET password_entered_ms = $2 WHERE device_id = $1",
        device_id,
        now_ms,
    )
    .execute(pool)
    .await?;
    Ok(())
}
