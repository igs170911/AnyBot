use anybot_domain::TaskStatus;
use anybot_foundation::FoundationError;
use anybot_protocol::RunEvent;

pub mod accounts;
pub mod auth_events;
pub mod config;
pub mod connect;
pub mod idempotency;
pub mod migrate;
pub mod rows;
pub mod sessions;

pub use config::{
    clear_config_keys, delete_config_row, read_config_rows, set_config_rows, write_config_row,
    write_config_rows,
};
pub use connect::{open_lazy, open_url, open_url_lazy, options, pool_options, probe, probe_pool};
pub use migrate::{Migrator, SchemaError, ensure_schema, product_migrations};
pub use rows::{account_count, public_tables, schema_marker_present};

pub type Pool = sqlx::PgPool;

#[derive(Debug, Clone)]
pub struct ServerRecord {
    pub event: RunEvent,
    pub status: TaskStatus,
}

#[must_use]
pub fn placeholder_missing_bootstrap() -> FoundationError {
    FoundationError::ConfigMissing("--bootstrap")
}
