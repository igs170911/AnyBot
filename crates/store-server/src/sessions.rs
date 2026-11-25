use crate::Pool;


#[derive(Debug)]
pub struct SessionRow {
    pub id: String,
    pub account: String,
    pub last_active_ms: i64,
    pub last_password_at_ms: i64,
}

pub async fn insert_session(
    pool: &Pool,
    id: &str,
    account: &str,
    key_hash: &str,
    label: &str,
    now_ms: i64,
) -> Result<(), sqlx::Error> {
    let account_row = sqlx::query!(
        "SELECT last_password_at_ms FROM accounts WHERE account = $1",
        account,
    )
    .fetch_one(pool)
    .await?;
    sqlx::query!(
        "INSERT INTO sessions (id, account, key_hash, label, created_at_ms, last_active_ms, \
         last_password_at_ms) VALUES ($1, $2, $3, $4, $5, $5, $6)",
        id,
        account,
        key_hash,
        label,
        now_ms,
        account_row.last_password_at_ms,
    )
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn find_session_by_key_hash(
    pool: &Pool,
    key_hash: &str,
) -> Result<Option<SessionRow>, sqlx::Error> {
    let row = sqlx::query!(
        "SELECT id, account, last_active_ms, last_password_at_ms \
         FROM sessions WHERE key_hash = $1",
        key_hash,
    )
    .fetch_optional(pool)
    .await?;
    let Some(row) = row else {
        return Ok(None);
    };
    Ok(Some(SessionRow {
        id: row.id,
        account: row.account,
        last_active_ms: row.last_active_ms,
        last_password_at_ms: row.last_password_at_ms,
    }))
}

pub async fn touch_session(pool: &Pool, id: &str, now_ms: i64) -> Result<(), sqlx::Error> {
    sqlx::query!(
        "UPDATE sessions SET last_active_ms = $2 WHERE id = $1",
        id,
        now_ms,
    )
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn list_sessions(pool: &Pool, account: &str) -> Result<Vec<SessionRow>, sqlx::Error> {
    let rows = sqlx::query!(
        "SELECT id, account, last_active_ms, last_password_at_ms \
         FROM sessions WHERE account = $1 ORDER BY last_active_ms",
        account,
    )
    .fetch_all(pool)
    .await?;
    let mut out = Vec::new();
    for row in rows {
        out.push(SessionRow {
            id: row.id,
            account: row.account,
            last_active_ms: row.last_active_ms,
            last_password_at_ms: row.last_password_at_ms,
        });
    }
    Ok(out)
}

pub async fn delete_sessions(pool: &Pool, ids: &[&str]) -> Result<(), sqlx::Error> {
    for id in ids {
        sqlx::query!("DELETE FROM sessions WHERE id = $1", id)
            .execute(pool)
            .await?;
    }
    Ok(())
}

pub async fn delete_sessions_for_account(
    pool: &Pool,
    account: &str,
    keep_id: Option<&str>,
) -> Result<(), sqlx::Error> {
    match keep_id {
        Some(keep) => {
            sqlx::query!(
                "DELETE FROM sessions WHERE account = $1 AND id <> $2",
                account,
                keep,
            )
            .execute(pool)
            .await?;
        }
        None => {
            sqlx::query!("DELETE FROM sessions WHERE account = $1", account)
                .execute(pool)
                .await?;
        }
    }
    Ok(())
}
