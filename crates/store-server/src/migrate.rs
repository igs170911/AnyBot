pub use sqlx::migrate::Migrator;

use sqlx::Row;

use crate::Pool;
use crate::rows::{public_tables, schema_marker_present};

#[must_use]
pub fn product_migrations() -> Migrator {
    sqlx::migrate!("../../migrations/server")
}

#[derive(Debug)]
pub enum SchemaError {
    ForeignData { tables: Vec<String> },
    MigrationFailed(String),
}

impl std::fmt::Display for SchemaError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SchemaError::ForeignData { tables } => {
                write!(f, "foreign tables in public schema: {}", tables.join(","))
            }
            SchemaError::MigrationFailed(reason) => write!(f, "migration failed: {reason}"),
        }
    }
}

pub async fn ensure_schema(pool: &Pool, migrator: &mut Migrator) -> Result<(), SchemaError> {
    let marked = schema_marker_present(pool)
        .await
        .map_err(|e| SchemaError::MigrationFailed(e.to_string()))?;
    if !marked {
        let tables = public_tables(pool)
            .await
            .map_err(|e| SchemaError::MigrationFailed(e.to_string()))?;
        if !tables.is_empty() {
            return Err(SchemaError::ForeignData { tables });
        }
    }
    run_migrator(migrator, pool).await
}

async fn run_migrator(migrator: &mut Migrator, pool: &Pool) -> Result<(), SchemaError> {
    let lock_key = migration_lock_key(pool).await?;
    let locked = acquire_migration_lock(pool, lock_key).await?;
    let mut conn = locked;
    migrator.set_locking(false);
    let result = migrator.run(pool).await;
    release_migration_lock(&mut conn).await?;
    match result {
        Ok(()) => Ok(()),
        Err(sqlx::migrate::MigrateError::VersionMissing(_version)) => Ok(()),
        Err(e) => Err(SchemaError::MigrationFailed(e.to_string())),
    }
}

async fn migration_lock_key(pool: &Pool) -> Result<i64, SchemaError> {
    let row = sqlx::query("SELECT hashtext(current_database())::bigint AS key")
        .fetch_one(pool)
        .await
        .map_err(|e| SchemaError::MigrationFailed(e.to_string()))?;
    let key = row
        .try_get::<i64, _>("key")
        .map_err(|e| SchemaError::MigrationFailed(e.to_string()))?;
    Ok(key)
}

async fn acquire_migration_lock(
    pool: &Pool,
    key: i64,
) -> Result<sqlx::pool::PoolConnection<sqlx::Postgres>, SchemaError> {
    let mut conn = pool
        .acquire()
        .await
        .map_err(|e| SchemaError::MigrationFailed(e.to_string()))?;
    let inner: &mut sqlx::PgConnection = &mut conn;
    sqlx::query("SELECT pg_advisory_lock($1)")
        .bind(key)
        .execute(inner)
        .await
        .map_err(|e| SchemaError::MigrationFailed(e.to_string()))?;
    Ok(conn)
}

async fn release_migration_lock(
    conn: &mut sqlx::pool::PoolConnection<sqlx::Postgres>,
) -> Result<(), SchemaError> {
    let inner: &mut sqlx::PgConnection = conn;
    sqlx::query("SELECT pg_advisory_unlock(hashtext(current_database())::bigint)")
        .execute(inner)
        .await
        .map_err(|e| SchemaError::MigrationFailed(e.to_string()))?;
    Ok(())
}
