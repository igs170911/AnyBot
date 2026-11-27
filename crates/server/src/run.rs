use anybot_store_server::Migrator;

use anybot_foundation::{Clock, MessageRef, ServerBootstrap};
use anybot_store_server::{Pool, SchemaError, SecretKeys};

use crate::boot::{BootError, Started, boot};

pub const CONNECT_RETRY_SECONDS: u64 = 5;
pub const CODE_SERVER_DB_CONNECT_RETRY: &str = "server-db-connect-retry";
pub const CODE_SERVER_DB_FOREIGN_DATA: &str = "server-db-foreign-data";
pub const CODE_SERVER_MIGRATION_FAILED: &str = "server-migration-failed";
pub const CODE_SERVER_MASTER_KEY_MISMATCH: &str = "server-master-key-mismatch";

#[derive(Debug)]
pub enum RunError {
    ForeignData,
    MigrationFailed(String),
    ConfigRead(String),
    ConfigInvalid(String),
    MasterKeyMismatch,
}

impl RunError {
    #[must_use]
    pub fn log_line(&self, bootstrap: &ServerBootstrap) -> MessageRef {
        match self {
            RunError::ForeignData => MessageRef::new(CODE_SERVER_DB_FOREIGN_DATA)
                .with_text("db_host", &bootstrap.host)
                .with_text("db_name", &bootstrap.database),
            RunError::MigrationFailed(reason) | RunError::ConfigRead(reason) => {
                MessageRef::new(CODE_SERVER_MIGRATION_FAILED).with_text("reason", reason)
            }
            RunError::ConfigInvalid(reason) => {
                MessageRef::new(crate::startup::CODE_SERVER_CONFIG_INVALID)
                    .with_text("reason", reason)
            }
            RunError::MasterKeyMismatch => MessageRef::new(CODE_SERVER_MASTER_KEY_MISMATCH),
        }
    }
}

#[must_use]
pub fn connect_retry_line(bootstrap: &ServerBootstrap, reason: &str) -> MessageRef {
    MessageRef::new(CODE_SERVER_DB_CONNECT_RETRY)
        .with_text("db_host", &bootstrap.host)
        .with_text("reason", reason)
        .with_int("seconds", CONNECT_RETRY_SECONDS.cast_signed())
}

pub async fn boot_database(
    pool: &Pool,
    clock: &dyn Clock,
    migrator: &mut Migrator,
    version: &str,
    keys: &SecretKeys,
) -> Result<Started, RunError> {
    match boot(pool, clock, migrator, version, keys).await {
        Ok(started) => Ok(started),
        Err(BootError::Schema(SchemaError::ForeignData { .. })) => Err(RunError::ForeignData),
        Err(BootError::Schema(SchemaError::MigrationFailed(reason))) => {
            Err(RunError::MigrationFailed(reason))
        }
        Err(BootError::ConfigRead(reason) | BootError::VersionWrite(reason)) => {
            Err(RunError::ConfigRead(reason))
        }
        Err(BootError::ConfigInvalid(reason)) => Err(RunError::ConfigInvalid(reason)),
        Err(BootError::MasterKeyMismatch) => Err(RunError::MasterKeyMismatch),
    }
}
