use anybot_foundation::{Secret, SecretBytes};

use crate::Pool;
use crate::keyed_hash::{HashPurpose, KeyedHash};
use crate::secret_keys::{SecretError, SecretKeys};
use crate::secret_seal::SecretPlace;

pub const TWO_STEP_TABLE: &str = "two_step";
pub const TWO_STEP_ACTIVE_COLUMN: &str = "secret_sealed";
pub const TWO_STEP_PENDING_COLUMN: &str = "pending_sealed";

#[derive(Debug)]
pub enum TwoStepStoreError {
    Sql(sqlx::Error),
    Secret(SecretError),
}

impl From<sqlx::Error> for TwoStepStoreError {
    fn from(err: sqlx::Error) -> Self {
        TwoStepStoreError::Sql(err)
    }
}

impl From<SecretError> for TwoStepStoreError {
    fn from(err: SecretError) -> Self {
        TwoStepStoreError::Secret(err)
    }
}

impl std::fmt::Display for TwoStepStoreError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TwoStepStoreError::Sql(err) => write!(f, "two-step storage failed: {err}"),
            TwoStepStoreError::Secret(err) => write!(f, "two-step secret failed: {err}"),
        }
    }
}

pub struct PendingSetup<'a> {
    pub account: &'a str,
    pub secret: &'a SecretBytes,
    pub recovery_codes: &'a [Secret],
    pub at_ms: i64,
}

fn place<'a>(column: &'a str, account: &'a str) -> SecretPlace<'a> {
    SecretPlace {
        table: TWO_STEP_TABLE,
        column,
        row: account,
    }
}

pub async fn two_step_enabled(pool: &Pool, account: &str) -> Result<bool, sqlx::Error> {
    sqlx::query_scalar!(
        r#"SELECT EXISTS(SELECT 1 FROM two_step WHERE account = $1 AND secret_sealed IS NOT NULL) AS "on!""#,
        account
    )
    .fetch_one(pool)
    .await
}

pub async fn open_active_secret(
    pool: &Pool,
    keys: &SecretKeys,
    account: &str,
) -> Result<Option<SecretBytes>, TwoStepStoreError> {
    let row = sqlx::query!(
        "SELECT secret_sealed FROM two_step WHERE account = $1",
        account
    )
    .fetch_optional(pool)
    .await?;
    let Some(sealed) = row.and_then(|row| row.secret_sealed) else {
        return Ok(None);
    };
    Ok(Some(
        keys.open(&place(TWO_STEP_ACTIVE_COLUMN, account), &sealed)?,
    ))
}

pub async fn open_pending_secret(
    pool: &Pool,
    keys: &SecretKeys,
    account: &str,
) -> Result<Option<SecretBytes>, TwoStepStoreError> {
    let row = sqlx::query!(
        "SELECT pending_sealed FROM two_step WHERE account = $1",
        account
    )
    .fetch_optional(pool)
    .await?;
    let Some(sealed) = row.and_then(|row| row.pending_sealed) else {
        return Ok(None);
    };
    Ok(Some(
        keys.open(&place(TWO_STEP_PENDING_COLUMN, account), &sealed)?,
    ))
}

pub async fn store_pending(
    pool: &Pool,
    keys: &SecretKeys,
    setup: &PendingSetup<'_>,
) -> Result<(), TwoStepStoreError> {
    let sealed = keys.seal(&place(TWO_STEP_PENDING_COLUMN, setup.account), setup.secret)?;
    let mut hashes = Vec::new();
    for code in setup.recovery_codes {
        hashes.push(keys.keyed_hash(HashPurpose::RecoveryCode, setup.account, code)?);
    }
    let mut tx = pool.begin().await?;
    sqlx::query!(
        "INSERT INTO two_step (account, pending_sealed, pending_at_ms) VALUES ($1, $2, $3) \
         ON CONFLICT (account) DO UPDATE SET pending_sealed = EXCLUDED.pending_sealed, \
         pending_at_ms = EXCLUDED.pending_at_ms",
        setup.account,
        sealed,
        setup.at_ms,
    )
    .execute(&mut *tx)
    .await?;
    sqlx::query!(
        "DELETE FROM two_step_recovery WHERE account = $1 AND pending",
        setup.account
    )
    .execute(&mut *tx)
    .await?;
    for (slot, hash) in hashes.iter().enumerate() {
        insert_recovery(&mut tx, setup.account, slot, hash).await?;
    }
    tx.commit().await?;
    Ok(())
}

async fn insert_recovery(
    conn: &mut sqlx::PgConnection,
    account: &str,
    slot: usize,
    hash: &KeyedHash,
) -> Result<(), TwoStepStoreError> {
    let slot = i16::try_from(slot).map_err(|e| SecretError::Unreadable(e.to_string()))?;
    sqlx::query!(
        "INSERT INTO two_step_recovery (account, pending, slot, algorithm, digest) \
         VALUES ($1, TRUE, $2, $3, $4)",
        account,
        slot,
        hash.algorithm,
        hash.digest,
    )
    .execute(&mut *conn)
    .await?;
    Ok(())
}

pub async fn confirm_pending(
    pool: &Pool,
    keys: &SecretKeys,
    account: &str,
    secret: &SecretBytes,
    at_ms: i64,
) -> Result<(), TwoStepStoreError> {
    let sealed = keys.seal(&place(TWO_STEP_ACTIVE_COLUMN, account), secret)?;
    let mut tx = pool.begin().await?;
    sqlx::query!(
        "UPDATE two_step SET secret_sealed = $2, enabled_at_ms = $3, pending_sealed = NULL, \
         pending_at_ms = NULL WHERE account = $1",
        account,
        sealed,
        at_ms,
    )
    .execute(&mut *tx)
    .await?;
    sqlx::query!(
        "DELETE FROM two_step_recovery WHERE account = $1 AND NOT pending",
        account
    )
    .execute(&mut *tx)
    .await?;
    sqlx::query!(
        "UPDATE two_step_recovery SET pending = FALSE WHERE account = $1 AND pending",
        account
    )
    .execute(&mut *tx)
    .await?;
    tx.commit().await?;
    Ok(())
}

pub async fn clear_two_step(pool: &Pool, account: &str) -> Result<(), sqlx::Error> {
    let mut tx = pool.begin().await?;
    sqlx::query!("DELETE FROM two_step WHERE account = $1", account)
        .execute(&mut *tx)
        .await?;
    sqlx::query!("DELETE FROM two_step_recovery WHERE account = $1", account)
        .execute(&mut *tx)
        .await?;
    sqlx::query!(
        "DELETE FROM two_step_used_steps WHERE account = $1",
        account
    )
    .execute(&mut *tx)
    .await?;
    sqlx::query!(
        "DELETE FROM two_step_challenges WHERE account = $1",
        account
    )
    .execute(&mut *tx)
    .await?;
    tx.commit().await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn places_differ_by_column_and_account() {
        let active = place(TWO_STEP_ACTIVE_COLUMN, "alice");
        assert_ne!(active, place(TWO_STEP_PENDING_COLUMN, "alice"));
        assert_ne!(active, place(TWO_STEP_ACTIVE_COLUMN, "bob"));
        assert_eq!(active.table, "two_step");
    }
}
