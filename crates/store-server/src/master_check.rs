use crate::Pool;
use crate::secret_keys::{CHECK_ALGORITHM, SecretKeys};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeyCheck {
    Matches,
    Absent,
}

#[derive(Debug)]
pub enum KeyCheckError {
    Mismatch,
    Unreadable(String),
}

impl std::fmt::Display for KeyCheckError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            KeyCheckError::Mismatch => f.write_str("master key does not match this database"),
            KeyCheckError::Unreadable(reason) => write!(f, "master key check unreadable: {reason}"),
        }
    }
}

pub async fn check_master_key(pool: &Pool, keys: &SecretKeys) -> Result<KeyCheck, KeyCheckError> {
    let present = sqlx::query_scalar!(
        r#"SELECT to_regclass('public.master_key_check') IS NOT NULL AS "present!""#
    )
    .fetch_one(pool)
    .await
    .map_err(|e| KeyCheckError::Unreadable(e.to_string()))?;
    if !present {
        return Ok(KeyCheck::Absent);
    }
    let row = sqlx::query!("SELECT algorithm, check_value FROM master_key_check WHERE id = 1")
        .fetch_optional(pool)
        .await
        .map_err(|e| KeyCheckError::Unreadable(e.to_string()))?;
    let Some(row) = row else {
        return Ok(KeyCheck::Absent);
    };
    if row.algorithm != CHECK_ALGORITHM {
        return Err(KeyCheckError::Unreadable(format!(
            "unknown check algorithm {}",
            row.algorithm
        )));
    }
    if keys.check_matches(&row.check_value) {
        Ok(KeyCheck::Matches)
    } else {
        Err(KeyCheckError::Mismatch)
    }
}

pub async fn record_master_key(
    pool: &Pool,
    keys: &SecretKeys,
    at_ms: i64,
) -> Result<(), sqlx::Error> {
    sqlx::query!(
        "INSERT INTO master_key_check (id, key_id, algorithm, check_value, recorded_at_ms) \
         VALUES (1, $1, $2, $3, $4) ON CONFLICT (id) DO NOTHING",
        keys.key_id_hex(),
        CHECK_ALGORITHM,
        keys.check_value(),
        at_ms,
    )
    .execute(pool)
    .await?;
    Ok(())
}
