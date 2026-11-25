pub use sqlx::migrate::Migrator;

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
    migrator.set_locking(false);
    match migrator.run(pool).await {
        Ok(()) => Ok(()),
        Err(sqlx::migrate::MigrateError::VersionMissing(_version)) => Ok(()),
        Err(e) => Err(SchemaError::MigrationFailed(e.to_string())),
    }
}
