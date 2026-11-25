use sqlx::Row;

use crate::Pool;

pub async fn schema_marker_present(pool: &Pool) -> Result<bool, sqlx::Error> {
    let row = sqlx::query("SELECT to_regclass('public.schema_marker') IS NOT NULL AS present")
        .fetch_one(pool)
        .await?;
    row.try_get("present")
}

pub async fn public_tables(pool: &Pool) -> Result<Vec<String>, sqlx::Error> {
    let rows = sqlx::query(
        "SELECT tablename FROM pg_tables WHERE schemaname = 'public' ORDER BY tablename",
    )
    .fetch_all(pool)
    .await?;
    let mut out = Vec::new();
    for row in rows {
        out.push(row.try_get("tablename")?);
    }
    Ok(out)
}

pub async fn account_count(pool: &Pool) -> Result<i64, sqlx::Error> {
    let row = sqlx::query("SELECT count(*) AS n FROM accounts")
        .fetch_one(pool)
        .await?;
    row.try_get("n")
}
