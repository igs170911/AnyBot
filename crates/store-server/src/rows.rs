use crate::Pool;

pub fn required<T>(value: Option<T>, what: &str) -> Result<T, sqlx::Error> {
    value.ok_or(sqlx::Error::Protocol(format!("missing {what} column")))
}

pub async fn schema_marker_present(pool: &Pool) -> Result<bool, sqlx::Error> {
    let row = sqlx::query!(
        "SELECT EXISTS (SELECT 1 FROM schema_marker WHERE app = 'anybot.server') AS present",
    )
    .fetch_one(pool)
    .await;
    match row {
        Ok(row) => required(row.present, "present"),
        Err(sqlx::Error::Database(db)) if db.message().contains("schema_marker") => Ok(false),
        Err(e) => Err(e),
    }
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

pub async fn min_applied_version(pool: &Pool) -> Result<i64, sqlx::Error> {
    let row = sqlx::query!("SELECT COALESCE(MIN(version), 0) AS low FROM _sqlx_migrations",)
        .fetch_one(pool)
        .await;
    match row {
        Ok(row) => required(row.low, "low"),
        Err(sqlx::Error::Database(db)) if db.message().contains("_sqlx_migrations") => Ok(0),
        Err(e) => Err(e),
    }
}

pub async fn max_applied_version(pool: &Pool) -> Result<i64, sqlx::Error> {
    let row = sqlx::query!("SELECT COALESCE(MAX(version), 0) AS high FROM _sqlx_migrations",)
        .fetch_one(pool)
        .await;
    match row {
        Ok(row) => required(row.high, "high"),
        Err(sqlx::Error::Database(db)) if db.message().contains("_sqlx_migrations") => Ok(0),
        Err(e) => Err(e),
    }
}

pub async fn account_count(pool: &Pool) -> Result<i64, sqlx::Error> {
    let row = sqlx::query!("SELECT count(*) AS n FROM accounts")
        .fetch_one(pool)
        .await?;
    required(row.n, "n")
}
