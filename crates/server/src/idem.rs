use axum::http::Method;
use axum::http::header::HeaderName;
use serde_json::Value;
use sha2::{Digest, Sha256};

use anybot_store_server::{Pool, idempotency};

pub const IDEMPOTENCY_HEADER: HeaderName = HeaderName::from_static("idempotency-key");
pub const IDEMPOTENCY_RETENTION_MS: u64 = 7 * 24 * 3600 * 1000;

pub const DID_IT_BODY: &str = "";

#[derive(Debug)]
pub struct Recorded {
    pub request_hash: String,
    pub status_code: i32,
    pub body: String,
}

#[must_use]
pub fn request_scope(method: &Method, path: &str, identity: &str) -> String {
    format!("{identity}|{method}|{path}")
}

#[must_use]
pub fn strip_secrets(body: &[u8]) -> Vec<u8> {
    let parsed: Value = serde_json::from_slice(body).unwrap_or(Value::Null);
    let cleaned = strip_value(&parsed);
    serde_json::to_vec(&cleaned).unwrap_or_default()
}

fn strip_value(value: &Value) -> Value {
    match value {
        Value::Object(map) => {
            let mut out = serde_json::Map::new();
            for (name, child) in map {
                if is_secret_field(name) {
                    continue;
                }
                out.insert(name.clone(), strip_value(child));
            }
            Value::Object(out)
        }
        Value::Null | Value::Bool(_) | Value::Number(_) | Value::String(_) | Value::Array(_) => {
            value.clone()
        }
    }
}

fn is_secret_field(name: &str) -> bool {
    matches!(name, "password" | "old" | "new" | "session_key" | "invite")
}

#[must_use]
pub fn body_digest(body: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(body);
    let out = hasher.finalize();
    crate::hex::to_hex(&out)
}

#[must_use]
pub fn safe_digest(body: &[u8]) -> String {
    body_digest(&strip_secrets(body))
}

pub async fn find_recorded(
    pool: &Pool,
    key: &str,
    scope: &str,
) -> Result<Option<Recorded>, sqlx::Error> {
    let row = idempotency::find_idempotency(pool, key, scope).await?;
    Ok(row.map(|row| Recorded {
        request_hash: row.request_hash,
        status_code: row.status_code,
        body: row.body,
    }))
}

pub async fn record_result(
    pool: &Pool,
    key: &str,
    scope: &str,
    request_hash: &str,
    status_code: u16,
    body: &str,
    at_ms: i64,
) -> Result<(), sqlx::Error> {
    idempotency::insert_idempotency(
        pool,
        key,
        scope,
        request_hash,
        i32::from(status_code),
        body,
        at_ms,
    )
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

    #[test]
    fn t_conn_post_1() {
        let with = br#"{"account":"bob","password":"secret12345"}"#;
        let without = br#"{"account":"bob"}"#;
        assert_eq!(safe_digest(with), safe_digest(without));
    }

    #[test]
    fn scope_binds_identity_method_path() {
        let a = request_scope(&Method::POST, "/v1/account/login", "bob");
        let b = request_scope(&Method::POST, "/v1/account/login", "alice");
        let c = request_scope(&Method::POST, "/v1/account/logout", "bob");
        assert_ne!(a, b);
        assert_ne!(a, c);
        assert_eq!(a, request_scope(&Method::POST, "/v1/account/login", "bob"));
    }
}
