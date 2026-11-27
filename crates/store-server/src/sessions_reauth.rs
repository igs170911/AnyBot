use crate::Pool;

pub async fn password_entered_ms(
    pool: &Pool,
    session_id: &str,
) -> Result<Option<i64>, sqlx::Error> {
    let row = sqlx::query!(
        "SELECT COALESCE(password_entered_ms, created_at_ms) AS entered FROM sessions WHERE id = $1",
        session_id,
    )
    .fetch_optional(pool)
    .await?;
    Ok(row.and_then(|row| row.entered))
}

pub async fn enter_password(pool: &Pool, session_id: &str, now_ms: i64) -> Result<(), sqlx::Error> {
    sqlx::query!(
        "UPDATE sessions SET password_entered_ms = $2 WHERE id = $1",
        session_id,
        now_ms,
    )
    .execute(pool)
    .await?;
    Ok(())
}
