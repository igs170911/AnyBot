use anybot_foundation::Secret;

use crate::Pool;
use crate::secret_keys::{SecretError, SecretKeys};
use crate::secret_seal::SecretPlace;

pub const PUSH_KEY_TABLE: &str = "push_server_key";
pub const PUSH_KEY_COLUMN: &str = "private_sealed";
pub const PUSH_KEY_ROW: &str = "vapid";

#[derive(Debug)]
pub enum PushStoreError {
    Sql(sqlx::Error),
    Secret(SecretError),
}

impl From<sqlx::Error> for PushStoreError {
    fn from(err: sqlx::Error) -> Self {
        PushStoreError::Sql(err)
    }
}

impl From<SecretError> for PushStoreError {
    fn from(err: SecretError) -> Self {
        PushStoreError::Secret(err)
    }
}

impl std::fmt::Display for PushStoreError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PushStoreError::Sql(err) => write!(f, "push storage failed: {err}"),
            PushStoreError::Secret(err) => write!(f, "push secret failed: {err}"),
        }
    }
}

#[must_use]
pub fn key_place() -> SecretPlace<'static> {
    SecretPlace {
        table: PUSH_KEY_TABLE,
        column: PUSH_KEY_COLUMN,
        row: PUSH_KEY_ROW,
    }
}

pub struct StoredPushKey {
    pub public_key: String,
    pub private: Secret,
}

pub async fn load(pool: &Pool, keys: &SecretKeys) -> Result<Option<StoredPushKey>, PushStoreError> {
    let row = sqlx::query!("SELECT public_key, private_sealed FROM push_server_key")
        .fetch_optional(pool)
        .await?;
    let Some(row) = row else {
        return Ok(None);
    };
    let private = keys.open_text(&key_place(), &row.private_sealed)?;
    Ok(Some(StoredPushKey {
        public_key: row.public_key,
        private,
    }))
}

pub async fn insert_if_absent(
    pool: &Pool,
    keys: &SecretKeys,
    public_key: &str,
    private: &Secret,
    now_ms: i64,
) -> Result<bool, PushStoreError> {
    let sealed = keys.seal_text(&key_place(), private)?;
    let done = sqlx::query!(
        "INSERT INTO push_server_key (public_key, private_sealed, created_at_ms) \
         VALUES ($1, $2, $3) ON CONFLICT DO NOTHING",
        public_key,
        sealed,
        now_ms,
    )
    .execute(pool)
    .await?;
    Ok(done.rows_affected() == 1)
}
