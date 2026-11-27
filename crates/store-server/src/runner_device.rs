use anybot_foundation::Secret;

use crate::Pool;
use crate::secret_keys::{SecretError, SecretKeys};
use crate::secret_seal::SecretPlace;

pub const RUNNER_TABLE: &str = "runner_service_device";
pub const RUNNER_KEY_COLUMN: &str = "key_sealed";
pub const RUNNER_ROW: &str = "runner-service";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RunnerDeviceRow {
    pub device_id: String,
    pub key_hash: String,
    pub created_at_ms: i64,
}

#[derive(Debug)]
pub enum RunnerStoreError {
    Sql(sqlx::Error),
    Secret(SecretError),
}

impl From<sqlx::Error> for RunnerStoreError {
    fn from(err: sqlx::Error) -> Self {
        RunnerStoreError::Sql(err)
    }
}

impl From<SecretError> for RunnerStoreError {
    fn from(err: SecretError) -> Self {
        RunnerStoreError::Secret(err)
    }
}

impl std::fmt::Display for RunnerStoreError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RunnerStoreError::Sql(err) => write!(f, "runner device storage failed: {err}"),
            RunnerStoreError::Secret(err) => write!(f, "runner device secret failed: {err}"),
        }
    }
}

#[must_use]
pub fn key_place() -> SecretPlace<'static> {
    SecretPlace {
        table: RUNNER_TABLE,
        column: RUNNER_KEY_COLUMN,
        row: RUNNER_ROW,
    }
}

pub struct NewRunnerDevice<'a> {
    pub device_id: &'a str,
    pub key: &'a Secret,
    pub key_hash: &'a str,
    pub now_ms: i64,
}

pub async fn insert_if_absent(
    pool: &Pool,
    keys: &SecretKeys,
    device: &NewRunnerDevice<'_>,
) -> Result<bool, RunnerStoreError> {
    let sealed = keys.seal_text(&key_place(), device.key)?;
    let done = sqlx::query!(
        "INSERT INTO runner_service_device (device_id, key_hash, key_sealed, created_at_ms) \
         VALUES ($1, $2, $3, $4) ON CONFLICT DO NOTHING",
        device.device_id,
        device.key_hash,
        sealed,
        device.now_ms,
    )
    .execute(pool)
    .await?;
    Ok(done.rows_affected() == 1)
}

pub async fn find(pool: &Pool) -> Result<Option<RunnerDeviceRow>, sqlx::Error> {
    sqlx::query_as!(
        RunnerDeviceRow,
        "SELECT device_id, key_hash, created_at_ms FROM runner_service_device",
    )
    .fetch_optional(pool)
    .await
}

pub async fn find_by_key_hash(
    pool: &Pool,
    key_hash: &str,
) -> Result<Option<RunnerDeviceRow>, sqlx::Error> {
    sqlx::query_as!(
        RunnerDeviceRow,
        "SELECT device_id, key_hash, created_at_ms FROM runner_service_device WHERE key_hash = $1",
        key_hash,
    )
    .fetch_optional(pool)
    .await
}

pub async fn open_key(pool: &Pool, keys: &SecretKeys) -> Result<Option<Secret>, RunnerStoreError> {
    let sealed = sqlx::query_scalar!("SELECT key_sealed FROM runner_service_device")
        .fetch_optional(pool)
        .await?;
    let Some(sealed) = sealed else {
        return Ok(None);
    };
    Ok(Some(keys.open_text(&key_place(), &sealed)?))
}
