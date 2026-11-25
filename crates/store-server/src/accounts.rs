use crate::Pool;

#[derive(Debug)]
pub struct AccountRow {
    pub account: String,
    pub password_hash: String,
    pub admin: bool,
    pub disabled: bool,
    pub created_at_ms: i64,
    pub last_password_at_ms: i64,
}

pub async fn insert_account(
    conn: &mut sqlx::PgConnection,
    account: &str,
    password_hash: &str,
    now_ms: i64,
) -> Result<bool, sqlx::Error> {
    sqlx::query!(
        "SELECT pg_advisory_xact_lock(5050505)",
    )
    .execute(&mut *conn)
    .await?;
    let row = sqlx::query!(
        "INSERT INTO accounts (account, password_hash, admin, created_at_ms, last_password_at_ms) \
         SELECT $1, $2, NOT EXISTS (SELECT 1 FROM accounts), $3, $3 \
         ON CONFLICT (account) DO NOTHING RETURNING admin",
        account,
        password_hash,
        now_ms,
    )
    .fetch_optional(&mut *conn)
    .await?;
    Ok(row.is_some_and(|row| row.admin))
}

pub async fn find_account(pool: &Pool, account: &str) -> Result<Option<AccountRow>, sqlx::Error> {
    let row = sqlx::query!(
        "SELECT account, password_hash, admin, disabled, created_at_ms, last_password_at_ms \
         FROM accounts WHERE account = $1",
        account,
    )
    .fetch_optional(pool)
    .await?;
    let Some(row) = row else {
        return Ok(None);
    };
    Ok(Some(AccountRow {
        account: row.account,
        password_hash: row.password_hash,
        admin: row.admin,
        disabled: row.disabled,
        created_at_ms: row.created_at_ms,
        last_password_at_ms: row.last_password_at_ms,
    }))
}

pub async fn set_password(
    pool: &Pool,
    account: &str,
    password_hash: &str,
    now_ms: i64,
) -> Result<(), sqlx::Error> {
    sqlx::query!(
        "UPDATE accounts SET password_hash = $2, last_password_at_ms = $3 WHERE account = $1",
        account,
        password_hash,
        now_ms,
    )
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn set_disabled(pool: &Pool, account: &str, disabled: bool) -> Result<(), sqlx::Error> {
    sqlx::query!(
        "UPDATE accounts SET disabled = $2 WHERE account = $1",
        account,
        disabled,
    )
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn account_count(pool: &Pool) -> Result<i64, sqlx::Error> {
    let row = sqlx::query!("SELECT count(*) AS total FROM accounts")
        .fetch_one(pool)
        .await?;
    row.total
        .ok_or(sqlx::Error::ColumnNotFound("total".to_owned()))
}

pub async fn list_accounts(pool: &Pool) -> Result<Vec<AccountRow>, sqlx::Error> {
    let rows = sqlx::query!(
        "SELECT account, password_hash, admin, disabled, created_at_ms, last_password_at_ms \
         FROM accounts ORDER BY created_at_ms",
    )
    .fetch_all(pool)
    .await?;
    let mut out = Vec::new();
    for row in rows {
        out.push(AccountRow {
            account: row.account,
            password_hash: row.password_hash,
            admin: row.admin,
            disabled: row.disabled,
            created_at_ms: row.created_at_ms,
            last_password_at_ms: row.last_password_at_ms,
        });
    }
    Ok(out)
}
