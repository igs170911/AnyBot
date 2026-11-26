use crate::Pool;

pub fn required<T>(value: Option<T>, what: &str) -> Result<T, sqlx::Error> {
    value.ok_or(sqlx::Error::Protocol(format!("missing {what} column")))
}

pub const UNDEFINED_TABLE: &str = "42P01";

#[must_use]
pub fn missing_relation(err: &sqlx::Error, relation: &str) -> bool {
    let sqlx::Error::Database(db) = err else {
        return false;
    };
    let undefined = db.code().is_some_and(|code| code == UNDEFINED_TABLE);
    if !undefined {
        return false;
    }
    match db.table() {
        Some(table) => table == relation,
        None => db.message().contains(relation),
    }
}

pub async fn schema_marker_present(pool: &Pool) -> Result<bool, sqlx::Error> {
    let row = sqlx::query!(
        "SELECT EXISTS (SELECT 1 FROM schema_marker WHERE app = 'anybot.server') AS present",
    )
    .fetch_one(pool)
    .await;
    match row {
        Ok(row) => required(row.present, "present"),
        Err(err) if missing_relation(&err, "schema_marker") => Ok(false),
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
        Err(err) if missing_relation(&err, "_sqlx_migrations") => Ok(0),
        Err(e) => Err(e),
    }
}

pub async fn max_applied_version(pool: &Pool) -> Result<i64, sqlx::Error> {
    let row = sqlx::query!("SELECT COALESCE(MAX(version), 0) AS high FROM _sqlx_migrations",)
        .fetch_one(pool)
        .await;
    match row {
        Ok(row) => required(row.high, "high"),
        Err(err) if missing_relation(&err, "_sqlx_migrations") => Ok(0),
        Err(e) => Err(e),
    }
}

pub async fn account_count(pool: &Pool) -> Result<i64, sqlx::Error> {
    let row = sqlx::query!("SELECT count(*) AS n FROM accounts")
        .fetch_one(pool)
        .await?;
    required(row.n, "n")
}
