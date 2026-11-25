use crate::Pool;

fn required<T>(value: Option<T>, what: &str) -> Result<T, sqlx::Error> {
    value.ok_or(sqlx::Error::Protocol(format!("missing {what} column")))
}

pub async fn schema_marker_present(pool: &Pool) -> Result<bool, sqlx::Error> {
    let row = sqlx::query!("SELECT to_regclass('public.schema_marker') IS NOT NULL AS present")
        .fetch_one(pool)
        .await?;
    required(row.present, "present")
}

pub async fn public_tables(pool: &Pool) -> Result<Vec<String>, sqlx::Error> {
    let rows = sqlx::query!(
        "SELECT tablename FROM pg_tables WHERE schemaname = 'public' ORDER BY tablename",
    )
    .fetch_all(pool)
    .await?;
    let mut out = Vec::new();
    for row in rows {
        out.push(required(row.tablename, "tablename")?);
    }
    Ok(out)
}

pub async fn account_count(pool: &Pool) -> Result<i64, sqlx::Error> {
    let row = sqlx::query!("SELECT count(*) AS n FROM accounts")
        .fetch_one(pool)
        .await?;
    required(row.n, "n")
}
