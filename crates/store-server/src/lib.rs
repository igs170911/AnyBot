use anybot_domain::TaskStatus;
use anybot_foundation::FoundationError;
use anybot_protocol::RunEvent;

pub mod accounts;
pub mod auth_events;
pub mod auth_events_kind;
pub mod computers;
pub mod config;
pub mod connect;
pub mod idempotency;
pub mod migrate;
pub mod pairing_codes;
pub mod rows;
pub mod sessions;
pub mod sessions_reauth;
pub mod streams;
pub mod streams_action;
pub mod streams_bots;
pub mod streams_card;
pub mod streams_settings;
pub use streams_bots as bots;

pub use config::{
    clear_config_keys, delete_config_row, read_config_rows, set_config_rows, write_config_row,
    write_config_rows,
};
pub use connect::{open_lazy, open_url, open_url_lazy, options, pool_options, probe, probe_pool};
pub use migrate::{Migrator, SchemaError, ensure_schema, product_migrations};
pub use rows::{
    account_count, max_applied_version, min_applied_version, public_tables, schema_marker_present,
};
pub use streams::{
    RecordRow, StreamRow, ensure_stream, find_record_by_device, insert_record, list_after,
    list_streams_of, load_stream, max_seq,
};
pub use streams_bots::{BotRow, ensure_bot, list_bots, load_bot, rename_bot, set_bot_deleted};

pub mod config_sealed;
pub mod keyed_hash;
pub mod master_check;
pub mod runner_device;
pub mod secret_keys;
pub mod secret_seal;
pub mod two_step;
pub mod two_step_challenges;
pub mod two_step_codes;
pub use keyed_hash::{HASH_ALGORITHM, HashPurpose, KeyedHash};
pub use master_check::{KeyCheck, KeyCheckError, check_master_key, record_master_key};
pub use secret_keys::{SecretError, SecretKeys};
pub use secret_seal::{SEAL_VERSION, SecretPlace};

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
