use crate::Pool;
use crate::secret_seal::SecretPlace;

pub const CONFIG_TABLE: &str = "server_config";
pub const CONFIG_SEALED_COLUMN: &str = "value_sealed";

#[must_use]
pub fn config_place(key: &str) -> SecretPlace<'_> {
    SecretPlace {
        table: CONFIG_TABLE,
        column: CONFIG_SEALED_COLUMN,
        row: key,
    }
}

pub async fn read_sealed_config(pool: &Pool, key: &str) -> Result<Option<Vec<u8>>, sqlx::Error> {
    let row = sqlx::query!("SELECT value_sealed FROM server_config WHERE key = $1", key)
        .fetch_optional(pool)
        .await?;
    Ok(row.and_then(|row| row.value_sealed))
}

pub async fn store_sealed_config(pool: &Pool, key: &str, sealed: &[u8]) -> Result<(), sqlx::Error> {
    sqlx::query!(
        "INSERT INTO server_config (key, value, value_sealed) VALUES ($1, '', $2) \
         ON CONFLICT (key) DO UPDATE SET value = '', value_sealed = EXCLUDED.value_sealed",
        key,
        sealed,
    )
    .execute(pool)
    .await?;
    Ok(())
}
