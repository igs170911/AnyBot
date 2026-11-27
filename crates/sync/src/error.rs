use anybot_store_client::StoreError;

#[derive(Debug, thiserror::Error)]
pub enum SyncError {
    #[error("local database error: {0}")]
    Store(#[from] StoreError),
    #[error("invalid configuration: {0}")]
    Config(String),
    #[error("record content cannot be encoded: {0}")]
    Encode(String),
    #[error("server reply cannot be understood: {0}")]
    Reply(String),
    #[error("the record is unknown to the outbox: {0}")]
    UnknownRecord(String),
}
