use std::time::Duration;

use sqlx::PgPool;

use crate::postgres::PgInstance;

const COPY_ATTEMPTS: u32 = 50;
const COPY_PAUSE: Duration = Duration::from_millis(100);

impl PgInstance {
    pub async fn snapshot_database(&self, database: &str, snapshot: &str) -> Result<(), String> {
        let pool = self.admin_pool().await?;
        let outcome = take_snapshot(&pool, database, snapshot).await;
        let reopened = run(
            &pool,
            &format!(
                "ALTER DATABASE {} WITH ALLOW_CONNECTIONS true",
                quoted(database)
            ),
        )
        .await;
        pool.close().await;
        outcome?;
        reopened
    }

    pub async fn restore_database(&self, database: &str, snapshot: &str) -> Result<(), String> {
        let pool = self.admin_pool().await?;
        let outcome = restore_from(&pool, database, snapshot).await;
        pool.close().await;
        outcome
    }
}

fn quoted(name: &str) -> String {
    format!("\"{}\"", name.replace('"', ""))
}

async fn run(pool: &PgPool, sql: &str) -> Result<(), String> {
    sqlx::query(sqlx::AssertSqlSafe(sql.to_owned()))
        .execute(pool)
        .await
        .map(|_done| ())
        .map_err(|e| format!("{sql} failed: {e}"))
}

async fn close_sessions(pool: &PgPool, database: &str) -> Result<(), String> {
    run(
        pool,
        &format!(
            "ALTER DATABASE {} WITH ALLOW_CONNECTIONS false",
            quoted(database)
        ),
    )
    .await?;
    sqlx::query(
        "SELECT pg_terminate_backend(pid) FROM pg_stat_activity \
         WHERE datname = $1 AND pid <> pg_backend_pid()",
    )
    .bind(database)
    .execute(pool)
    .await
    .map(|_done| ())
    .map_err(|e| format!("terminate sessions of {database} failed: {e}"))
}

async fn create_from_template(pool: &PgPool, new: &str, template: &str) -> Result<(), String> {
    let sql = format!(
        "CREATE DATABASE {} TEMPLATE {}",
        quoted(new),
        quoted(template)
    );
    let mut last = String::new();
    for _attempt in 0..COPY_ATTEMPTS {
        match run(pool, &sql).await {
            Ok(()) => return Ok(()),
            Err(e) => last = e,
        }
        tokio::time::sleep(COPY_PAUSE).await;
    }
    Err(last)
}

async fn take_snapshot(pool: &PgPool, database: &str, snapshot: &str) -> Result<(), String> {
    run(
        pool,
        &format!("DROP DATABASE IF EXISTS {} WITH (FORCE)", quoted(snapshot)),
    )
    .await?;
    close_sessions(pool, database).await?;
    create_from_template(pool, snapshot, database).await?;
    run(
        pool,
        &format!(
            "ALTER DATABASE {} WITH ALLOW_CONNECTIONS false",
            quoted(snapshot)
        ),
    )
    .await
}

async fn restore_from(pool: &PgPool, database: &str, snapshot: &str) -> Result<(), String> {
    close_sessions(pool, database).await?;
    run(
        pool,
        &format!("DROP DATABASE IF EXISTS {} WITH (FORCE)", quoted(database)),
    )
    .await?;
    create_from_template(pool, database, snapshot).await?;
    run(
        pool,
        &format!(
            "ALTER DATABASE {} WITH ALLOW_CONNECTIONS true",
            quoted(database)
        ),
    )
    .await
}
