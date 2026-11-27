use anybot_foundation as _;

pub mod error;
pub mod model;
pub mod open;
pub mod outbox;
pub mod outbox_rows;
pub mod purge;
pub mod recover;
pub mod recover_read;
pub mod replica;
pub mod replica_rows;
pub mod settings;
pub mod store;

pub use error::StoreError;
pub use model::{
    ACCOUNT_STREAM, ApplyOutcome, LocalRecord, NewOutbox, OutboxRow, OutboxState, Progress,
    bot_stream, written_by_server,
};
pub use open::{DATABASE_FILE, OpenOutcome};
pub use recover::{Recovery, RecoveryReason};
pub use settings::SettingValue;
pub use store::{Pool, Store};

#[cfg(test)]
mod tests;
