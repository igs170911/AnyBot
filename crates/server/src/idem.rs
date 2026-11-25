use axum::http::header::HeaderName;
use sha2::{Digest, Sha256};

use anybot_store_server::{Pool, idempotency};

pub const IDEMPOTENCY_HEADER: HeaderName = HeaderName::from_static("idempotency-key");
pub const IDEMPOTENCY_RETENTION_MS: u64 = 7 * 24 * 3600 * 1000;

#[derive(Debug)]
pub struct Recorded {
    pub request_hash: String,
    pub status_code: i32,
    pub body: String,
}

#[must_use]
pub fn body_digest(body: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(body);
    let out = hasher.finalize();
    crate::hex::to_hex(&out)
}

pub async fn find_recorded(pool: &Pool, key: &str) -> Result<Option<Recorded>, sqlx::Error> {
    let row = idempotency::find_idempotency(pool, key).await?;
    Ok(row.map(|row| Recorded {
        request_hash: row.request_hash,
        status_code: row.status_code,
        body: row.body,
    }))
}

pub async fn record_result(
    pool: &Pool,
    key: &str,
    request_hash: &str,
    status_code: u16,
    body: &str,
    at_ms: i64,
) -> Result<(), sqlx::Error> {
    idempotency::insert_idempotency(pool, key, request_hash, i32::from(status_code), body, at_ms)
        .await
}

pub async fn prune_idempotency(pool: &Pool, now_ms: u64) -> Result<(), sqlx::Error> {
    let Ok(floor) = i64::try_from(now_ms.saturating_sub(IDEMPOTENCY_RETENTION_MS)) else {
        return Ok(());
    };
    idempotency::prune_idempotency_before(pool, floor).await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn digest_distinguishes_payloads() {
        assert_eq!(body_digest(b"a"), body_digest(b"a"));
        assert_ne!(body_digest(b"a"), body_digest(b"b"));
    }
}
