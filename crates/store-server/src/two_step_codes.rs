use anybot_foundation::Secret;

use crate::Pool;
use crate::keyed_hash::{HashPurpose, KeyedHash};
use crate::secret_keys::SecretKeys;
use crate::two_step::TwoStepStoreError;

pub async fn claim_step(
    pool: &Pool,
    account: &str,
    step: i64,
    at_ms: i64,
    reuse_floor_ms: i64,
) -> Result<bool, sqlx::Error> {
    let row = sqlx::query_scalar!(
        "INSERT INTO two_step_used_steps (account, step, used_at_ms) VALUES ($1, $2, $3) \
         ON CONFLICT (account, step) DO UPDATE SET used_at_ms = EXCLUDED.used_at_ms \
         WHERE two_step_used_steps.used_at_ms < $4 RETURNING step",
        account,
        step,
        at_ms,
        reuse_floor_ms,
    )
    .fetch_optional(pool)
    .await?;
    Ok(row.is_some())
}

pub async fn use_recovery_code(
    pool: &Pool,
    keys: &SecretKeys,
    account: &str,
    code: &Secret,
    at_ms: i64,
) -> Result<bool, TwoStepStoreError> {
    let rows = sqlx::query!(
        "SELECT slot, algorithm, digest FROM two_step_recovery \
         WHERE account = $1 AND NOT pending AND used_at_ms IS NULL ORDER BY slot",
        account
    )
    .fetch_all(pool)
    .await?;
    let mut matched = None;
    for row in rows {
        let stored = KeyedHash {
            algorithm: row.algorithm,
            digest: row.digest,
        };
        let hit = keys.keyed_hash_matches(HashPurpose::RecoveryCode, account, code, &stored)?;
        if hit && matched.is_none() {
            matched = Some(row.slot);
        }
    }
    let Some(slot) = matched else {
        return Ok(false);
    };
    let done = sqlx::query!(
        "UPDATE two_step_recovery SET used_at_ms = $3 \
         WHERE account = $1 AND NOT pending AND slot = $2 AND used_at_ms IS NULL",
        account,
        slot,
        at_ms,
    )
    .execute(pool)
    .await?;
    Ok(done.rows_affected() == 1)
}

pub async fn prune_used_steps_before(pool: &Pool, floor_ms: i64) -> Result<u64, sqlx::Error> {
    let done = sqlx::query!(
        "DELETE FROM two_step_used_steps WHERE used_at_ms < $1",
        floor_ms
    )
    .execute(pool)
    .await?;
    Ok(done.rows_affected())
}
