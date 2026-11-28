use anybot_server::Pool;

use crate::seed::ADMIN_ACCOUNT;

const OWNED_TABLES: [&str; 5] = [
    "DELETE FROM sessions WHERE account = $1",
    "DELETE FROM bots WHERE account = $1",
    "DELETE FROM computers WHERE account = $1",
    "DELETE FROM mailbox_letters WHERE account = $1",
    "DELETE FROM two_step WHERE account = $1",
];

const ACCOUNT_ROW: &str = "DELETE FROM accounts WHERE account = $1";

pub async fn delete_account(pool: &Pool, account: &str) -> Result<bool, String> {
    if account == ADMIN_ACCOUNT {
        return Err("the testenv admin account cannot be deleted".to_owned());
    }
    for statement in OWNED_TABLES {
        sqlx::query(statement)
            .bind(account)
            .execute(pool)
            .await
            .map_err(|e| format!("delete owned rows failed: {e}"))?;
    }
    let removed = sqlx::query(ACCOUNT_ROW)
        .bind(account)
        .execute(pool)
        .await
        .map_err(|e| format!("delete account failed: {e}"))?;
    Ok(removed.rows_affected() > 0)
}
